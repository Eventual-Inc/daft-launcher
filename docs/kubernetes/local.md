# Local Kubernetes Development Setup

This guide walks you through setting up a local Kubernetes cluster for Daft development.

## Prerequisites

- Docker Desktop installed and running
- `kubectl` CLI tool installed
- `helm` installed
- One of the following local Kubernetes solutions:
  - Kind (Recommended)
  - Minikube
  - Docker Desktop's built-in Kubernetes

## Option 1: Using Kind (Recommended)

1. Install Kind:
   ```bash
   # On macOS with Homebrew
   brew install kind

   # On Linux
   curl -Lo ./kind https://kind.sigs.k8s.io/dl/v0.20.0/kind-linux-amd64
   chmod +x ./kind
   sudo mv ./kind /usr/local/bin/kind
   ```

2. Create a cluster:
   ```bash
   # For Apple Silicon (M1, M2, M3):
   kind create cluster --name daft-dev --config - <<EOF
   kind: Cluster
   apiVersion: kind.x-k8s.io/v1alpha4
   nodes:
   - role: control-plane
     image: kindest/node:v1.27.3@sha256:3966ac761ae0136263ffdb6cfd4db23ef8a83cba8a463690e98317add2c9ba72
   - role: worker
     image: kindest/node:v1.27.3@sha256:3966ac761ae0136263ffdb6cfd4db23ef8a83cba8a463690e98317add2c9ba72
   EOF

   # For x86/AMD64:
   kind create cluster --name daft-dev
   ```

3. Verify the cluster is running:
   ```bash
   kubectl cluster-info
   ```

## Option 2: Using Minikube

1. Install Minikube:
   ```bash
   # On macOS with Homebrew
   brew install minikube

   # On Linux
   curl -LO https://storage.googleapis.com/minikube/releases/latest/minikube-linux-amd64
   sudo install minikube-linux-amd64 /usr/local/bin/minikube
   ```

2. Start Minikube:
   ```bash
   # For Apple Silicon (M1, M2, M3):
   minikube start --driver=docker --kubernetes-version=v1.27.3 \
     --cpus=4 --memory=8192 --disk-size=20g

   # For x86/AMD64:
   minikube start --cpus=4 --memory=8192
   ```

3. Verify the cluster is running:
   ```bash
   minikube status
   ```

## Installing Ray and Daft

Once your local cluster is running, follow the [Ray Installation Guide](./ray-installation.md) to:
1. Install KubeRay Operator
2. Deploy Ray cluster
3. Install Daft
4. Set up port forwarding
5. Submit test jobs

> **Note**: For Apple Silicon (M1, M2, M3) machines, make sure to use the ARM64-specific Ray image as specified in the installation guide.

## Resource Requirements

Local Kubernetes clusters need sufficient resources to run Ray and Daft effectively:

- Minimum requirements:
  - 4 CPU cores
  - 8GB RAM
  - 20GB disk space

- Recommended:
  - 8 CPU cores
  - 16GB RAM
  - 40GB disk space

You can adjust these in Docker Desktop's settings or when starting Minikube.

## Troubleshooting

### Resource Issues
- If pods are stuck in `Pending` state:
  - For Docker Desktop: Increase resources in Docker Desktop settings
  - For Minikube: Start with more resources: `minikube start --cpus 6 --memory 12288`

### Architecture Issues
- For Apple Silicon users:
  - Ensure you're using ARM64-compatible images
  - Check Docker Desktop is running in native ARM64 mode
  - Verify Kubernetes components are ARM64-compatible

## Cleanup

To delete your local cluster:

```bash
# For Kind
kind delete cluster --name daft-dev

# For Minikube
minikube delete
``` 