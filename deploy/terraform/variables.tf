variable "aws_region" {
  type        = string
  description = "AWS Region to deploy resources into"
  default     = "us-east-1"
}

variable "instance_type" {
  type        = string
  description = "EC2 Instance type (t3.small is recommended for minimum memory footprint)"
  default     = "t3.small"
}

variable "my_ip" {
  type        = string
  description = "Your local public IP address (CIDR format, e.g., '203.0.113.50/32') to authorize SSH and Redpanda access. Run `curl ifconfig.me` to find it."
}

variable "public_key_path" {
  type        = string
  description = "Local path to your SSH public key used for accessing the EC2 instance"
  default     = "~/.ssh/id_rsa.pub"
}

variable "project_name" {
  type        = string
  description = "Project name tag for resource organization"
  default     = "riskfabric"
}
