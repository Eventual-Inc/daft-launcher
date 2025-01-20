# Cloud Provider Kubernetes Setup

This guide covers using Ray and Daft with managed Kubernetes services from major cloud providers.

## Prerequisites

### General Requirements
- `kubectl` installed and configured
- `helm` installed
- A running Kubernetes cluster in one of the following cloud providers:
  - Amazon Elastic Kubernetes Service (EKS)
  - Google Kubernetes Engine (GKE)
  - Azure Kubernetes Service (AKS)

### Cloud-Specific Requirements

#### For AWS EKS
- AWS CLI installed and configured
- Access to an existing EKS cluster
- `kubectl` configured for your EKS cluster:
  ```bash
  aws eks update-kubeconfig --name your-cluster-name --region your-region
  ```

#### For Google GKE
- Google Cloud SDK installed
- Access to an existing GKE cluster
- `kubectl` configured for your GKE cluster:
  ```bash
  gcloud container clusters get-credentials your-cluster-name --zone your-zone
  ```

#### For Azure AKS
- Azure CLI installed
- Access to an existing AKS cluster
- `kubectl` configured for your AKS cluster:
  ```bash
  az aks get-credentials --resource-group your-resource-group --name your-cluster-name
  ```

## Installing Ray and Daft

Once your cloud Kubernetes cluster is running and `kubectl` is configured, follow the [Ray Installation Guide](./ray-installation.md) to:
1. Install KubeRay Operator
2. Deploy Ray cluster
3. Install Daft
4. Set up port forwarding
5. Submit test jobs

> **Note**: For cloud providers, you'll typically use x86/AMD64 images unless you're specifically using ARM-based instances (like AWS Graviton).