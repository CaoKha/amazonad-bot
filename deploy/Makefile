# ── Target configuration ──────────────────────────────────────────
# Pop!_OS System76 laptop (local network)
LOCAL_USER ?= YOUR_POPOS_USERNAME
LOCAL_IP   ?= system76.local
LOCAL      := $(LOCAL_USER)@$(LOCAL_IP)

# Oracle Cloud VM (optional alternative)
CLOUD_USER ?= ubuntu
CLOUD_IP   ?= YOUR_ORACLE_VM_IP
CLOUD      := $(CLOUD_USER)@$(CLOUD_IP)

BINARY     := monitoring-the-situation
REMOTE_DIR := /opt/ads-monitor
MUSL_TARGET := x86_64-unknown-linux-musl

.PHONY: deploy-local setup-local logs-local status-local sleep-status \
        build-cloud deploy-cloud setup-cloud logs-cloud status-cloud

# ── Pop!_OS (System76) ────────────────────────────────────────────
deploy-local:
	@echo '==> Building on System76 laptop...'
	ssh $(LOCAL) 'cd $(REMOTE_DIR) && git pull 2>/dev/null || true'
	ssh $(LOCAL) 'source $$HOME/.cargo/env && cd $(REMOTE_DIR) && cargo build --release 2>&1'
	ssh $(LOCAL) 'sudo cp $(REMOTE_DIR)/target/release/$(BINARY) $(REMOTE_DIR)/$(BINARY)'
	ssh $(LOCAL) 'sudo chown monitor:monitor $(REMOTE_DIR)/$(BINARY)'
	ssh $(LOCAL) 'sudo systemctl restart $(BINARY)'
	@echo '==> Deployed. Run: make logs-local'

setup-local:
	@echo '==> Setting up System76 laptop...'
	ssh $(LOCAL) 'command -v cargo >/dev/null 2>&1 || (curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y)'
	ssh $(LOCAL) 'sudo useradd -r -s /sbin/nologin monitor 2>/dev/null || true'
	ssh $(LOCAL) 'sudo mkdir -p $(REMOTE_DIR) && sudo chown $(LOCAL_USER):$(LOCAL_USER) $(REMOTE_DIR)'
	scp ../deploy/ads-monitor.service $(LOCAL):/tmp/
	ssh $(LOCAL) 'sudo cp /tmp/ads-monitor.service /etc/systemd/system/$(BINARY).service && sudo systemctl daemon-reload && sudo systemctl enable $(BINARY)'
	ssh $(LOCAL) 'sudo systemctl mask sleep.target suspend.target hibernate.target hybrid-sleep.target'
	ssh $(LOCAL) 'sudo sed -i "s/#HandleLidSwitch=suspend/HandleLidSwitch=ignore/" /etc/systemd/logind.conf'
	ssh $(LOCAL) 'grep -q "HandleLidSwitch=ignore" /etc/systemd/logind.conf || echo "HandleLidSwitch=ignore" | sudo tee -a /etc/systemd/logind.conf'
	ssh $(LOCAL) 'sudo systemctl restart systemd-logind'
	@echo '==> Setup complete. Now:'
	@echo '    scp .env $(LOCAL):$(REMOTE_DIR)/.env'
	@echo '    scp config.toml $(LOCAL):$(REMOTE_DIR)/config.toml'
	@echo '    make deploy-local'

logs-local:
	ssh $(LOCAL) 'sudo journalctl -u $(BINARY) -f --no-pager'

status-local:
	ssh $(LOCAL) 'sudo systemctl status $(BINARY)'

sleep-status:
	@echo '==> Sleep/suspend status:'
	ssh $(LOCAL) 'sudo systemctl status sleep.target suspend.target hibernate.target 2>&1 | grep -E "Loaded|Active"'

# ── Oracle Cloud ──────────────────────────────────────────────────
build-cloud:
	cargo build --release --target $(MUSL_TARGET)

deploy-cloud: build-cloud
	scp target/$(MUSL_TARGET)/release/$(BINARY) $(CLOUD):/tmp/$(BINARY)
	ssh $(CLOUD) 'sudo systemctl stop $(BINARY) 2>/dev/null || true'
	ssh $(CLOUD) 'sudo cp /tmp/$(BINARY) $(REMOTE_DIR)/ && sudo chmod +x $(REMOTE_DIR)/$(BINARY)'
	ssh $(CLOUD) 'sudo chown monitor:monitor $(REMOTE_DIR)/$(BINARY)'
	ssh $(CLOUD) 'sudo systemctl start $(BINARY)'
	@echo '==> Deployed to Oracle. Run: make logs-cloud'

setup-cloud:
	ssh $(CLOUD) 'sudo useradd -r -s /sbin/nologin monitor 2>/dev/null || true'
	ssh $(CLOUD) 'sudo mkdir -p $(REMOTE_DIR) && sudo chown monitor:monitor $(REMOTE_DIR)'
	scp ../deploy/ads-monitor.service $(CLOUD):/tmp/
	ssh $(CLOUD) 'sudo cp /tmp/ads-monitor.service /etc/systemd/system/$(BINARY).service && sudo systemctl daemon-reload && sudo systemctl enable $(BINARY)'

logs-cloud:
	ssh $(CLOUD) 'sudo journalctl -u $(BINARY) -f --no-pager'

status-cloud:
	ssh $(CLOUD) 'sudo systemctl status $(BINARY)'
