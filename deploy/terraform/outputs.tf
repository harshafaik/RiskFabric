output "public_ip" {
  value       = aws_eip.eip.public_ip
  description = "The static Elastic IP address of the RiskFabric server"
}

output "ssh_connection_command" {
  value       = "ssh -i <path_to_private_key> ubuntu@${aws_eip.eip.public_ip}"
  description = "The command to SSH connect to the EC2 server (replace <path_to_private_key> with the path to your private key file, e.g., ~/.ssh/id_rsa)"
}

output "streamlit_dashboard_url" {
  value       = "http://${aws_eip.eip.public_ip}:8501"
  description = "The public web address of your Streamlit monitoring dashboard"
}

output "local_streamer_env_var" {
  value       = "export KAFKA_BOOTSTRAP_SERVERS=${aws_eip.eip.public_ip}:9092"
  description = "Run this command in your local terminal before launching the Rust stream generator to target the cloud stack"
}
