# On-Premises Kubernetes Setup

This guide covers setting up Ray and Daft on self-managed Kubernetes clusters.

## Prerequisites

Before proceeding with Ray and Daft installation, ensure you have:

- A running Kubernetes cluster (v1.16+)
- `kubectl` installed and configured with access to your cluster
- `helm` installed
- Load balancer solution configured if needed

## Verifying Cluster Requirements

1. Check Kubernetes version:
   ```bash
   kubectl version --short
   ```

2. Verify cluster nodes:
   ```bash
   kubectl get nodes
   ```

## Installing Ray and Daft

Once your on-premises Kubernetes cluster is ready, follow the [Cloud Provider Setup Guide](./cloud.md#installing-ray-common-steps-for-all-providers) for:
- Installing Ray using Helm
- Installing Daft on the Ray cluster
- Configuring and using daft-cli

The installation steps are identical regardless of where your Kubernetes cluster is running.