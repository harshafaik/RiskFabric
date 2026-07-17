# RiskFabric Cloud Deployment Guide (Podman + Grafana Hybrid Setup)

This guide documents the steps to deploy the RiskFabric scoring stack (Kafka/Redpanda, ClickHouse, Redis, Scorer, Grafana Dashboard) to AWS EC2 using Terraform, while streaming transactions locally (Option B) using **Podman** containers.

---

## 🛠️ Prerequisites

1. **AWS CLI** configured on your local machine with appropriate IAM permissions (AdministratorAccess or permissions to manage EC2, VPC, EIP, and SG).
   ```bash
   aws configure
   ```
2. **Terraform** (>= 1.0.0) installed locally.
3. An **SSH Key Pair** at `~/.ssh/id_rsa.pub` (or customized path). If you don't have one, generate it with:
   ```bash
   ssh-keygen -t rsa -b 4096
   ```

---

## 🚀 Step 1: Provision Infrastructure with Terraform

1. Navigate to the terraform directory:
   ```bash
   cd deploy/terraform
   ```
2. Copy the variable file template:
   ```bash
   cp terraform.tfvars.example terraform.tfvars
   ```
3. Open `terraform.tfvars` and edit:
   * Run `curl ifconfig.me` in your terminal to get your local public IP.
   * Put this IP in `my_ip` with a `/32` suffix (e.g. `"203.0.113.50/32"`). This secures your cloud instance so only you can SSH and produce messages to Kafka.
4. Initialize Terraform and apply the configuration:
   ```bash
   terraform init
   terraform apply
   ```
5. Confirm the apply step by typing `yes`. Once completed, Terraform will output your server's **Elastic IP** and instructions.

---

## 📦 Step 2: Deploy the Stack on EC2 using Podman

Since your repository is already tracked on GitHub, the cleanest way to transfer your files to the server is using `rsync` from your local machine.

### Sync files via rsync
Run these commands from your **local repository root** (replace `<PUBLIC_IP>` with the Elastic IP from Terraform's output):
```bash
# Sync repository files (excluding heavy build directories and git metadata)
rsync -avz --exclude 'target' --exclude '.git' --exclude 'data/references/ref_merchants.parquet' -e "ssh -i ~/.ssh/id_rsa" ./ ubuntu@<PUBLIC_IP>:/home/ubuntu/RiskFabric

# Sync gitignored model assets
rsync -avz -e "ssh -i ~/.ssh/id_rsa" ./models/ ubuntu@<PUBLIC_IP>:/home/ubuntu/RiskFabric/models/

# Sync gitignored reference data
rsync -avz -e "ssh -i ~/.ssh/id_rsa" ./data/references/ ubuntu@<PUBLIC_IP>:/home/ubuntu/RiskFabric/data/references/
```

### Build the Scorer Base Image:
Because Podman runs in a daemonless environment, we first build the base Python container image locally on the EC2 host.
1. SSH into the server using the connection command from Terraform's outputs:
   ```bash
   ssh -i ~/.ssh/id_rsa ubuntu@<PUBLIC_IP>
   ```
2. Navigate to the project folder:
   ```bash
   cd RiskFabric
   ```
3. Build the custom ML scoring container image:
   ```bash
   podman build -t localhost/riskfabric_dlt -f Dockerfile.dlt .
   ```

### Launch the Stack:
1. Start the Podman Compose stack, passing the `PUBLIC_IP` environment variable so Kafka/Redpanda advertises your public address to your local machine:
   ```bash
   PUBLIC_IP=<PUBLIC_IP> podman-compose up -d
   ```
2. Verify all containers are up and running:
   ```bash
   podman ps
   ```

---

## ⚡ Step 3: Stream Transactions Locally

Now that the backend is running in the cloud, you can run the simulator on your local machine and stream transactions into the cloud Kafka topic.

1. Open a terminal on your **local machine** and navigate to your local `RiskFabric` directory.
2. Set the target bootstrap servers to point to the cloud EC2 instance (using the Elastic IP):
   ```bash
   export KAFKA_BOOTSTRAP_SERVERS=<PUBLIC_IP>:9092
   ```
3. Run the streaming script:
   ```bash
   cargo run --release --bin stream
   ```
   You should see outputs like:
   `Connecting to Kafka at <PUBLIC_IP>:9092`
   `Sent 100 transactions`

---

## 📊 Step 4: Monitor via Grafana Dashboard

1. Open your browser and navigate to `http://<PUBLIC_IP>:3000`.
2. The pre-provisioned Grafana dashboard will load, showing live-updating metrics for:
   * Total Scored Transactions.
   * Fraud Detection Rates.
   * Real-time Pipeline Latencies (Feat extraction, prediction, DB write).
   * Live Fraud Queue showing incoming flagged incidents.

---

## 🛑 Step 5: Clean Up

To avoid incurring charges on AWS when you are done benchmarking:

1. SSH into the EC2 instance and stop the containers:
   ```bash
   podman-compose down
   ```
2. On your local machine, navigate to `deploy/terraform/` and run:
   ```bash
   terraform destroy
   ```
3. Confirm by typing `yes`. Terraform will cleanly delete the EC2 instance, Elastic IP, Security Groups, and VPC, ensuring you are billed $0.
