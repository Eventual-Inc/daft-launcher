# Kubernetes Setup for Daft

> **Note**: This documentation is housed in the `daft-launcher` repository while the Kubernetes approach is being reviewed. Once finalized, these docs will be copied to the separate documentation repository.

This directory contains guides for setting up Ray and Daft on various Kubernetes environments:

- [Local Development](./local.md) - Setting up a local Kubernetes cluster for development
- [Cloud Providers](./cloud.md) - Instructions for EKS, GKE, and AKS setups
- [On-Premises](./on-prem.md) - Guide for on-premises Kubernetes deployments

## Prerequisites

Before using `daft-launcher` with Kubernetes, you must:
1. Have a running Kubernetes cluster (local, cloud-managed, or on-premise)
2. Install and configure Ray on your Kubernetes cluster
3. Install Daft on your cluster

Please follow the appropriate guide above for your environment. 