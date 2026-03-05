# Oracle Cloud Free VM — Deployment Guide

> **Use this if your System76 laptop is off or unavailable.** Oracle's Always Free tier includes two AMD VMs that never expire — no cost, ever.

---

## Step 1 — Create an Oracle Cloud Account

Go to [https://cloud.oracle.com](https://cloud.oracle.com) and sign up.

A credit card is required for identity verification only — Oracle does not charge it on the Always Free tier.

---

## Step 2 — Create a VM

1. Open the Oracle Cloud Console → **Compute** → **Instances** → **Create Instance**
2. Select the **"Always Free Eligible"** AMD Micro shape (`VM.Standard.E2.1.Micro`)
3. Choose **Ubuntu 22.04** as the image
4. Under **Add SSH keys**, upload your public key (`~/.ssh/oracle_key.pub`)
5. Click **Create**

Wait a few minutes for the instance to provision.

---

## Step 3 — Note the Public IP

Once the instance is running, copy the **Public IP Address** from the instance details page. You'll use this everywhere below.

---

## Step 4 — SSH In

```bash
ssh -i ~/.ssh/oracle_key ubuntu@YOUR_IP
```

If you don't have a key yet, generate one first:

```bash
ssh-keygen -t ed25519 -C 'mac-to-oracle' -f ~/.ssh/oracle_key
```

Then go back to Step 2 and upload `~/.ssh/oracle_key.pub`.

---

## Step 5 — Install the Cross-Compiler on Mac

The daemon is cross-compiled on your Mac and uploaded as a static binary.

```bash
# Install musl cross-compiler (macOS)
brew install FiloSottile/musl-cross/musl-cross

# Add the Rust target
rustup target add x86_64-unknown-linux-musl
```

---

## Step 6 — Setup and Deploy

From the project root on your Mac:

```bash
# Edit deploy/Makefile: set CLOUD_IP to your Oracle VM's public IP
# CLOUD_IP ?= YOUR_ORACLE_VM_IP

# Run first-time setup (creates monitor user, installs systemd service)
make setup-cloud

# Copy your secrets
scp .env ubuntu@YOUR_IP:/opt/ads-monitor/.env
scp config.toml ubuntu@YOUR_IP:/opt/ads-monitor/config.toml

# Build and deploy
make deploy-cloud
make status-cloud
```

---

## Step 7 — Verify

```bash
make logs-cloud
```

You should see `Starting monitoring loop` in the output. The daemon is now running 24/7.

---

## Firewall Note

The default Oracle Cloud Security List allows inbound SSH (port 22). **No additional ports or firewall rules are needed** — the daemon only makes outbound HTTPS calls to the Amazon Ads API and Telegram API.

If you ever lock yourself out of SSH, go to Oracle Console → **Networking** → **Virtual Cloud Networks** → your VCN → **Security Lists** → **Default Security List** and verify there's an ingress rule for TCP port 22 from `0.0.0.0/0`.

---

## Updating the Daemon

After pushing changes to git:

```bash
make deploy-cloud   # Cross-compiles, uploads, restarts
make logs-cloud     # Verify clean restart
```
