# BYOC (Bring Your Own Cluster) Mode Setup for Daft

This directory contains guides for setting up Ray and Daft on various Kubernetes environments for BYOC mode:

- [Local Development](./local.md) - Setting up a local Kubernetes cluster for development
- [Cloud Providers](./cloud.md) - Instructions for EKS, GKE, and AKS setups
- [On-Premises](./on-prem.md) - Guide for on-premises Kubernetes deployments

## Prerequisites

Before using `daft-launcher` in BYOC mode with Kubernetes, you must:
1. Have a running Kubernetes cluster (local, cloud-managed, or on-premise)
2. Install and configure Ray on your Kubernetes cluster
3. Install Daft on your cluster

Please follow the appropriate guide above for your environment. 