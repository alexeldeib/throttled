# throttled

Simple IOPS and throughput monitoring for Azure VMs.

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
