#![allow(dead_code)]

use async_std::stream;
use eyre::{eyre, Context, Result};
use futures::prelude::*;
use prometheus::{Encoder, GaugeVec, Opts, Registry, TextEncoder};
use relative_path::RelativePath;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

mod imds;
mod iostat;
mod kube;
mod oauth;
mod resource;

use oauth::OAuthResponse;
use resource::{Disk, VirtualMachine};

fn main() -> Result<()> {
    smol::run(async {
        let azure_json = kube::new()?;
        let meta = imds::new().await?;

        let token: OAuthResponse = oauth::get_msi_token(&azure_json.user_assigned_identity_id)
            .await
            .wrap_err_with(|| "failed to get msi token")?;

        let vm_size = meta.compute.vm_size;
        let location = azure_json.location;

        let os_disk_size = meta
            .compute
            .storage_profile
            .os_disk
            .disk_size_gb
            .parse::<u64>()?;

        let os_disk_storage_type = meta
            .compute
            .storage_profile
            .os_disk
            .managed_disk
            .storage_account_type;

        let mut vm_skus = resource::list_skus(&token.access_token, &azure_json.subscription_id)
            .await?
            .value
            .into_iter()
            .filter(|sku| sku.resource_type == "virtualMachines")
            .filter(|sku| sku.locations.len() > 0 && sku.locations[0] == location)
            .filter(|sku| sku.name == vm_size)
            .map(|sku| VirtualMachine::try_from(sku))
            .collect::<Result<Vec<VirtualMachine>>>()?;

        match vm_skus.len() {
            1 => {}
            n => {
                return Err(eyre!(
                    "expected single matching vm sku but found {}. matches: {:#?}",
                    &n,
                    &vm_skus,
                ))
            }
        }

        if vm_skus.len() < 1 {
            return Err(eyre!("no matching sku found for virtual machine"));
        }

        if vm_skus.len() > 1 {
            return Err(eyre!("multiple matching vm skus found: {:#?}", vm_skus));
        }

        let vm_sku = vm_skus.pop().unwrap();

        let disk_skus = resource::list_skus(&token.access_token, &azure_json.subscription_id)
            .await?
            .value
            .into_iter()
            .filter(|sku| sku.resource_type == "disks")
            .filter(|sku| sku.locations.len() > 0 && sku.locations[0] == location)
            .filter(|res| res.tier != Some("Ultra".to_string())) // Need to support ultra, it has different range-based structure
            .map(|sku| Disk::try_from(sku))
            .collect::<Result<Vec<Disk>>>()?;

        let mut os_disk_sku = disk_skus
            .iter()
            .filter(|sku| &sku.storage_account_type == &os_disk_storage_type)
            .filter(|sku| &os_disk_size > &sku.min_size_gb && &os_disk_size <= &sku.max_size_gb)
            .cloned()
            .collect::<Vec<Disk>>();

        if os_disk_sku.len() < 1 {
            return Err(eyre!(
                "no matching sku found for os disk with size: {}, storage type: {}",
                &os_disk_size,
                &os_disk_storage_type,
            ));
        }

        if os_disk_sku.len() > 1 {
            return Err(eyre!(
                "multiple matching sku found for os disk with size: {}, storage type: {}",
                &os_disk_size,
                &os_disk_storage_type,
            ));
        }

        // let paths = fs::read_dir("/dev/disk/azure/scsi1/lun0")?
        //     .into_iter()
        //     .filter_map(Result::ok)
        //     .map(|e| e.path())
        //     .map(|e| e.to_owned().into_os_string().into_string())
        //     .filter_map(Result::ok)
        //     .collect::<Vec<String>>();

        // println!("{:#?}", &paths);

        let mut skus: HashMap<String, Disk> = HashMap::new();
        let os_disk_sku = os_disk_sku.pop().unwrap();
        skus.insert("/dev/sda".to_string(), os_disk_sku);

        for disk in meta.compute.storage_profile.data_disks {
            let size = disk.disk_size_gb.parse::<u64>()?;
            let storage_account_type = disk.managed_disk.storage_account_type;
            let disk_sku = get_disk_sku(&disk_skus, &size, &storage_account_type)?;

            // TODO(ace): clean this up...maybe shell to readlink -f?
            // normalization without following the symlink seems
            // strangely difficult.
            let device_file = fs::read_link(format!("/dev/disk/azure/scsi1/lun{}", &disk.lun))
                .wrap_err_with(|| "failed to read link")?;

            let device_file: PathBuf =
                RelativePath::new(&format!("/dev/disk/azure/scsi1/{}", device_file.display(),))
                    .normalize()
                    .to_path("/");

            let device_file = match device_file.to_owned().into_os_string().into_string() {
                Err(e) => {
                    return Err(eyre!(
                        "failed to convert path to friendly udev label: {:?}; err: {:?}",
                        &device_file,
                        &e,
                    ))
                }
                Ok(s) => s,
            };

            skus.insert(device_file, disk_sku);
        }
        // let paths = fs::read_dir("/sys/block")?
        //     .into_iter()
        //     .filter_map(Result::ok)
        //     .map(|e| e.path())
        //     .map(|e| e.to_owned().into_os_string().into_string())
        //     .filter_map(Result::ok)
        //     .filter(|e| !e.contains("loop"))
        //     .collect::<Vec<String>>();

        // println!("{:#?}", &paths);

        // for path in paths {
        //     println!("Name: {}", path.unwrap().path().display())
        // }

        // println!("{:#?}", &meta.compute);

        // let iterated: Vec<Resource> = skus
        //     .clone()
        //     .into_iter()
        //     .filter(|sku| sku.locations[0] == azure_json.location)
        //     .filter(|sku| sku.resource_type == "disks")
        //     .filter(|sku| sku.size.is_some())
        //     .filter(|sku| sku.size.clone().unwrap().contains("P80"))
        //     .collect();

        // .into_iter()
        // .filter(resource::with_disk_size(os_disk_size))
        // .collect::<Vec<Disk>>();

        println!("vm: {:#?}", vm_sku);
        println!("disks: {:#?}", skus);
        // println!("disks: {:#?}", disk_skus);

        let r = Registry::new();

        let iops_limit_gauge_opts = Opts::new(
            "iops_limit_gauge",
            "Gauge from 0 to 1 representing percentage of iops limit saturated for a given device or host",
        );

        let bandwidth_limits_sopts = Opts::new(
            "bandwidth_limit_gauge",
            "Gauge from 0 to 1 representing percentage of bandwidth limit saturated by current throughput for a given device or host",
        );

        let iops_gauge_opts = Opts::new(
            "iops_gauge",
            "Gauge counting point-in-time IOPS for a given device or host",
        );

        let bandwidth_opts = Opts::new(
            "bandwidth_gauge",
            "Gauge counting point-in-time throughput for a given device or host",
        );

        let iops_limit_gauge = GaugeVec::new(iops_limit_gauge_opts, &["device"])?;
        let bandwidth_limit_gauge = GaugeVec::new(bandwidth_limits_sopts, &["device"])?;
        let iops_gauge = GaugeVec::new(iops_gauge_opts, &["device"])?;
        let bandwidth_gauge = GaugeVec::new(bandwidth_opts, &["device"])?;

        r.register(Box::new(iops_gauge.clone()))?;
        r.register(Box::new(bandwidth_gauge.clone()))?;
        r.register(Box::new(iops_limit_gauge.clone()))?;
        r.register(Box::new(bandwidth_limit_gauge.clone()))?;

        let mut interval = stream::interval(Duration::from_secs(1));

        while let Some(_) = interval.next().await {
            println!("tick");
            let mut total_iops: f64 = 0.0;
            let mut total_throughput: f64 = 0.0;

            for (disk, stats) in iostat::new()?.iter() {
                let iops =
                    stats.reads_per_second + stats.writes_per_second + stats.flushes_per_second;
                let throughput = stats.read_kilo_bytes_per_second
                    + stats.write_kilo_bytes_per_second
                    + stats.discard_kilo_bytes_per_second;

                total_iops += iops;
                total_throughput += throughput;

                iops_gauge.with_label_values(&[disk]).set(iops);
                bandwidth_gauge.with_label_values(&[disk]).set(throughput);

                let disk_limit = skus.get(disk);
                if disk_limit.is_none() {
                    println!("no sku information stored for disk: {}", &disk);
                    continue;
                }
                let disk_limit = disk_limit.unwrap();

                let iops_ratio = iops / disk_limit.max_iops as f64;
                let throughput_ratio = throughput / disk_limit.max_bandwidth as f64;
                iops_limit_gauge.with_label_values(&[disk]).set(iops_ratio);
                bandwidth_limit_gauge
                    .with_label_values(&[disk])
                    .set(throughput_ratio);
            }

            let total_iops_ratio = total_iops / vm_sku.max_iops as f64;
            let total_throughput_ratio = total_throughput / vm_sku.max_bandwidth as f64;

            iops_limit_gauge
                .with_label_values(&["TOTAL"])
                .set(total_iops_ratio);
            bandwidth_limit_gauge
                .with_label_values(&["TOTAL"])
                .set(total_throughput_ratio);
            iops_gauge.with_label_values(&["TOTAL"]).set(total_iops);
            bandwidth_gauge
                .with_label_values(&["TOTAL"])
                .set(total_throughput);

            // Gather the metrics.
            let mut buffer = vec![];
            let encoder = TextEncoder::new();
            let metric_families = r.gather();
            encoder.encode(&metric_families, &mut buffer)?;

            // Output to the standard output.
            println!("{}", String::from_utf8(buffer)?);
        }

        Ok(())
    })
}

fn get_disk_sku(skus: &Vec<Disk>, size: &u64, storage_account_type: &str) -> Result<Disk> {
    let mut filtered = skus
        .iter()
        .filter(|sku| &sku.storage_account_type == storage_account_type)
        .filter(|sku| size > &sku.min_size_gb && size <= &sku.max_size_gb)
        .cloned()
        .collect::<Vec<Disk>>();

    if filtered.len() < 1 {
        return Err(eyre!(
            "no matching sku found for disk with size: {}, storage type: {}",
            &size,
            &storage_account_type,
        ));
    }

    if filtered.len() > 1 {
        return Err(eyre!(
            "multiple matching sku found for disk with size: {}, storage type: {}, matches: {:#?}",
            &size,
            &storage_account_type,
            &filtered,
        ));
    }

    Ok(filtered.pop().unwrap())
}
