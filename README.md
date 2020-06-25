# throttled

Simple IOPS and throughput monitoring for Azure VMs. Discovers all disk
and machine limits through instance metadata and exposes current
utilization as a percentage of total uncached limits. Note that ratios
may be greater than 1 with caching enabled, and may briefly peak above 1
before encountering throttling.

## Log output

```
vm: VirtualMachine {
    name: "Standard_D8s_v3",
    location: "westus2",
    max_iops: 12800,
    max_bandwidth: 201326592,
}
disks: {
    "/dev/sdd": Disk {
        location: "westus2",
        storage_account_type: "Premium_LRS",
        min_size_gb: 256,
        max_size_gb: 512,
        max_iops: 2300,
        max_bandwidth: 157286400,
    },
    "/dev/sda": Disk {
        location: "westus2",
        storage_account_type: "Premium_LRS",
        min_size_gb: 512,
        max_size_gb: 1024,
        max_iops: 5000,
        max_bandwidth: 209715200,
    },
    "/dev/sdc": Disk {
        location: "westus2",
        storage_account_type: "Premium_LRS",
        min_size_gb: 64,
        max_size_gb: 128,
        max_iops: 500,
        max_bandwidth: 104857600,
    },
}
```

## Prometheus metrics

(note that iops and bandwidth as raw numbers are available through many
sources, they are here only as a sanity check against the ratios since
they need to be collected anyway for calculation. It's possible to only
expose the limits as fully static metrics to optimize resource usage).
```
# HELP bandwidth_gauge Gauge counting point-in-time throughput for a given device or host
# TYPE bandwidth_gauge gauge
bandwidth_gauge{device="/dev/loop0"} 0
bandwidth_gauge{device="/dev/sda"} 10
bandwidth_gauge{device="/dev/sdb"} 0
bandwidth_gauge{device="/dev/sdc"} 0
bandwidth_gauge{device="/dev/sdd"} 0
bandwidth_gauge{device="TOTAL"} 10
# HELP bandwidth_limit_gauge Gauge from 0 to 1 representing percentage of uncached bandwidth limit saturated by current throughput for a given device or host
# TYPE bandwidth_limit_gauge gauge
bandwidth_limit_gauge{device="/dev/sda"} 0.0000000476837158203125
bandwidth_limit_gauge{device="/dev/sdc"} 0
bandwidth_limit_gauge{device="/dev/sdd"} 0
bandwidth_limit_gauge{device="TOTAL"} 0.00000004967053731282552
# HELP iops_gauge Gauge counting point-in-time IOPS for a given device or host
# TYPE iops_gauge gauge
iops_gauge{device="/dev/loop0"} 0
iops_gauge{device="/dev/sda"} 1.5
iops_gauge{device="/dev/sdb"} 0
iops_gauge{device="/dev/sdc"} 0
iops_gauge{device="/dev/sdd"} 0
iops_gauge{device="TOTAL"} 1.5
# HELP iops_limit_gauge Gauge from 0 to 1 representing percentage of uncached iops limit saturated for a given device or host
# TYPE iops_limit_gauge gauge
iops_limit_gauge{device="/dev/sda"} 0.0003
iops_limit_gauge{device="/dev/sdc"} 0
iops_limit_gauge{device="/dev/sdd"} 0
iops_limit_gauge{device="TOTAL"} 0.0001171875
```

## what it does

The daemon uses IMDS to identify the VM size, OS disk size, and OS disk
storage type. IMDS also provides the size, LUN, and storage types for
all data disks. The SKUs API provides all virtual machine and
disk SKUs with cabilities such as IOPS/bandwidth. Currently, we require
MSI on the VM to authenticate to the SKUs API.

By joining IMDS and SKU data on VM size and storage type, we can
identify VM-level IOPS and bandwidth limits.

By joining IMDS and SKU data on disk size and storage type, we can
identify disk-level IOPS and bandwidth limits.

We use the LUNs from data disks to map to friendly udev labels like /dev/sdX.
This is preferable for metrics capture.

We assume /dev/sda and /dev/sdb are always the OS disk and temporary
disk for linux VMs. This is used only in its absence: We assume iostat
will collect this device and don't manually try to map it using udev rules.

After discovering all limits, the daemon polls iostat (or any similar
method) periodically. It exposes a metrics endpoint Prometheus-style
with the hostname and device name, along with a gauge equal to the
percent of IOPS or bandwidth limit. We use friendly udev labels mapped
from LUNs to attach to each metric.

## development

The main development dependencies are cargo and docker. A skaffold
configuration exists for testing against an existing Azure cluster.

build:
```
cargo build
```

docker build:
```
docker build . -t yourtag
```

continous development with skaffold:
```
skaffold dev
```
