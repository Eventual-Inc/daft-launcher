# Installing Ray on Kubernetes

This guide covers the common steps for installing Ray on Kubernetes using KubeRay, regardless of where your cluster is running (local, cloud, or on-premise).

## Prerequisites
- A running Kubernetes cluster
- `kubectl` configured with the correct context
- `helm` installed

## Installation Steps

1. Add the KubeRay Helm repository:
   ```bash
   helm repo add kuberay https://ray-project.github.io/kuberay-helm/
   helm repo update
   ```

2. Install KubeRay Operator:
   ```bash
   helm install kuberay-operator kuberay/kuberay-operator
   ```

3. Create a values file (`values.yaml`):
   ```yaml
   head:
     args: ["sudo apt-get update && sudo apt-get install -y curl; curl -LsSf https://astral.sh/uv/install.sh | sh; export PATH=$HOME/.local/bin:$PATH; uv pip install --system getdaft"]
   worker:
     args: ["sudo apt-get update && sudo apt-get install -y curl; curl -LsSf https://astral.sh/uv/install.sh | sh; export PATH=$HOME/.local/bin:$PATH; uv pip install --system getdaft"]

   rayCluster:
     headGroupSpec:
       template:
         spec:
           containers:
             - name: ray-head
               image: rayproject/ray:2.40.0-py310  # Use the desired Python version
               command: ["ray", "start", "--head"]
     workerGroupSpecs:
         template:
           spec:
             containers:
               - name: ray-worker
                 image: rayproject/ray:2.40.0-py310  # Same image to ensure compatibility
   ```

4. Install Ray Cluster:
   
   For Apple Silicon (M1, M2, M3, M4) or other ARM64 processors (AWS Graviton, etc.):
   ```bash
   helm install raycluster kuberay/ray-cluster --version 1.2.2 \
     --set 'image.tag=2.40.0-py310-aarch64' \
     -f values.yaml
   ```

   For x86/AMD64 processors:
   ```bash
   helm install raycluster kuberay/ray-cluster --version 1.2.2 \
     -f values.yaml
   ```

6. Verify the installation:
   ```bash
   kubectl get pods
   ```

## Accessing Ray

### Port Forwarding
To access the Ray dashboard and submit jobs, set up port forwarding:
```bash
kubectl port-forward service/raycluster-kuberay-head-svc 8265:8265
```

### Ray Dashboard
Once port forwarding is set up, access the dashboard at:
http://localhost:8265

### Submitting Jobs
You can submit Ray jobs using the following command:
```bash
ray job submit --address http://localhost:8265 -- python -c "import ray; import daft; ray.init(); print(ray.cluster_resources())"
```

## Troubleshooting

1. Check pod status:
   ```bash
   kubectl get pods
   kubectl describe pod <pod-name>
   ```

2. View pod logs:
   ```bash
   kubectl logs <pod-name>
   ```

3. Common issues:
   - If pods are stuck in `Pending` state, check resource availability
   - If pods are `CrashLoopBackOff`, check the logs for errors
   - For ARM64 issues, ensure you're using the correct image tag with `-aarch64` suffix 