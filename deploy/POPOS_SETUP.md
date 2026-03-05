# Pop!_OS (System76 Laptop) — 24/7 Deployment Guide

Run the ads monitor daemon on your System76 laptop with Pop!_OS. The laptop stays on with the lid closed, and a systemd service keeps the daemon running across reboots.

---

## A — Enable SSH on the System76 Laptop

Run these commands **on the laptop itself** (open a terminal):

```bash
sudo apt install openssh-server -y
sudo systemctl enable --now ssh
ip addr show | grep 'inet ' | grep -v 127.0.0.1
# Note your LAN IP (e.g. 192.168.1.42)
```

You can also try mDNS: `hostname.local` (e.g. `system76.local`). If mDNS doesn't work on your network, use the LAN IP directly.

---

## B — Set Up Passwordless SSH from Mac

Run these commands **on your Mac**:

```bash
# Generate a dedicated key pair
ssh-keygen -t ed25519 -C 'mac-to-system76' -f ~/.ssh/system76_key

# Copy the public key to the laptop
ssh-copy-id -i ~/.ssh/system76_key.pub YOUR_USER@192.168.1.42

# Test the connection
ssh -i ~/.ssh/system76_key YOUR_USER@192.168.1.42 'echo connected'
```

Add this to `~/.ssh/config` on your Mac for convenience:

```
Host system76
    HostName 192.168.1.42
    User YOUR_USER
    IdentityFile ~/.ssh/system76_key
```

---

## C — Keep the Laptop Always-On

Run these commands **on the laptop** (or via SSH):

```bash
# Disable all sleep/suspend/hibernate targets
sudo systemctl mask sleep.target suspend.target hibernate.target hybrid-sleep.target

# Ignore lid close events
echo 'HandleLidSwitch=ignore' | sudo tee -a /etc/systemd/logind.conf
sudo systemctl restart systemd-logind

# Optional: disable screen blanking (saves a few watts if you disable the display instead)
gsettings set org.gnome.desktop.session idle-delay 0
gsettings set org.gnome.settings-daemon.plugins.power sleep-inactive-ac-timeout 0
```

Verify sleep is disabled:

```bash
sudo systemctl status sleep.target suspend.target hibernate.target
# All should show "masked"
```

> **Tip:** Keep the laptop plugged in at all times. Close the lid — it won't sleep.

---

## D — First-Time Setup from Mac

1. Edit `deploy/Makefile` — set `LOCAL_USER` and `LOCAL_IP` to match your laptop:

   ```makefile
   LOCAL_USER ?= nick
   LOCAL_IP   ?= 192.168.1.42
   ```

2. Run the setup target:

   ```bash
   make setup-local
   ```

   This installs Rust, creates the `monitor` user, copies the systemd service, and disables sleep.

3. Copy your secrets to the laptop:

   ```bash
   scp .env user@laptop:/opt/ads-monitor/.env
   scp config.toml user@laptop:/opt/ads-monitor/config.toml
   ```

4. Deploy and verify:

   ```bash
   make deploy-local
   make status-local   # Should show: active (running)
   ```

---

## E — Updating the Daemon

After pushing changes to git:

```bash
make deploy-local   # Pulls latest code, rebuilds, restarts the service
make logs-local     # Verify clean restart — look for "Starting monitoring loop"
```

That's it. The Makefile handles `git pull`, `cargo build --release`, binary copy, and `systemctl restart` in one command.
