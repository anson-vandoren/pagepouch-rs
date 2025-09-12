# DEPLOYMENT.md

## Production Environment Discovery

This document outlines the deployment strategy and runbook for PagePouch production environment.

### Environment Questions (To Be Answered)

**Infrastructure:**

- [x] What type of hosting platform are you using? (VPS, cloud provider, bare metal, container platform, etc.)
  - Digital Ocean droplet for now. Possibly switching to Hetzner in the future
- [x] What operating system is running in production?
  - Ubuntu 22.x
- [x] Do you have root/sudo access on the production server?
  - Yes
- [x] Are you using any containerization (Docker, Podman) or orchestration (Kubernetes, Docker Compose)?
  - Docker is available but I'd strongly prefer not to use it. Kubernetes is not an option.

**Current Deployment Status:**

- [x] Is PagePouch already deployed in production, or is this a new deployment?
  - There is an entirely different system in place right now for handling bookmark management. This will essentially be a new deployment.
- [x] If already deployed, how is it currently being deployed? (manual copy, git pull, CI/CD, etc.)
  - The existing solution (not pagepouch) is managed by manually creating docker images, manually pushing them to GitHub packages, manually pulling from the DO droplet, manually restarting the docker container with the new image.
- [x] What domain/URL will PagePouch be accessible from?
  - Eventually https://links.ansonvandoren.com, but right now that's hosting the existing solution. We'll probably deploy to a different subdomain, or else I'll just buy a new domain name and set up nginx on the droplet (it's already hosting many other domain names/services)
  - I just bought pagepouch.com, so let's do that one.

**Database:**

- [x] Where will the SQLite database be stored in production?
  - Locally on the droplet. Probably in /opt/pagepouch/ somewhere but I don't have a strong preference
- [x] Do you need database backups? If so, what's your preferred backup strategy?
  - The whole droplet is backed up regularly. A separate PagePouch database backup is maybe something to look at later but not needed now.
- [x] Are there any data persistence requirements or volume mounts needed?
  - Depends on how we choose to deploy, I guess. I think my ideal case is the app runs as a standalone binary and stores its database in a location specified by config

**Web Server & Networking:**

- [x] Will you use a reverse proxy (nginx, Apache, Caddy, Traefik) in front of PagePouch?
  - Yes, nginx
- [x] Do you need SSL/TLS certificates? If so, do you prefer Let's Encrypt, manual certificates, or cloud provider certificates?
  - Yes, we will. Let's Encrypt is already handling other certificates on the box through certbot's nginx module. Let's stick with that
- [x] What port should PagePouch bind to in production? (currently defaults to 8888)
  - I'll have to check what's currently available. Let's say 8888 for now (for no good reason other than it's the first one I know probably isn't used)
  - 8888 is free, let's do that
- [x] Are there any firewall or security group configurations needed?
  - We don't need to worry about this, it's already handled for the whole box

**Process Management:**

- [x] How should PagePouch run as a service? (systemd, supervisor, container restart policy, etc.)
  - I guess probably systemd
- [x] What user should the PagePouch process run as?
  - There is a `blog` user that we can use
- [x] Do you need log rotation or centralized logging?
  - prefer /var/log/. We don't really need more than say the last 2 weeks, at most

**Build & Deployment:**

- [x] Do you prefer to build the Rust binary locally and upload it, or build on the production server?
  - Either built locally or built in a GitHub Actions runner. Probably locally 'cause GHA is slow AF
- [x] Are you using any CI/CD platform? (GitHub Actions, GitLab CI, Jenkins, etc.)
  - I can use GitHub Actions if we have a good reason to. I've used it before and it's... OK. But slow. And tedious.
- [x] Do you want automated deployments on git push, or manual deployment triggers?
  - Prefer automated deployments either on a tag or on a release. I already have [webhook](https://github.com/adnanh/webhook) running on the box handling some other deployments. I also have a Telegram bot ready to receive notifications for new deployments, which I'd like to do here if possible
  - https://ansonvandoren.com/hooks/${service-specific}, but we'll have to make the last path segment and configure webhook for it. Everything under `/hooks/` should get to `webhook` listener though. Despite being on ansonvandoren.com, it will still be able to manage PagePouch - same DO droplet
  - I guess we can do actual releases - maybe that's easier since it'll already have a binary associated with it? If we stick with `webhook` we'll need to build a configuration and a script for it to run which will pull the release binary, I guess

**Configuration & Secrets:**

- [x] How do you want to manage environment variables and secrets in production?
  - We should only need one base secret, I think. I _think_ I'm fine with that just being on disk on the DO droplet. Happy to hear arguments to the contrary, though. I don't want anything complicated or expensive though.
- [x] Do you need different configuration for production vs development?
  - Nope, just a single prod deployment is fine. For now, this is used only by myself and a small handful of friends.

**Monitoring & Updates:**

- [x] Do you want health checks or monitoring for the application?
  - I already use [uptime-kuma](https://github.com/louislam/uptime-kuma) for basic health checks on other domains/sites/services and probably can use it here as well
- [x] How do you prefer to handle application updates? (rolling updates, blue-green, maintenance windows)
  - Downtime of a few seconds is fine. There is only a single instance running, ever, though.
- [x] Do you need any alerting if the service goes down?
  - Uptime-kuma can handle this already for me via Telegram

## Deployment Runbook

### Overview

- **Domain**: https://pagepouch.com
- **Port**: 8888 (internal)
- **User**: blog
- **Database**: /opt/pagepouch/pagepouch.db
- **Binary**: /opt/pagepouch/pagepouch
- **Config**: /opt/pagepouch/.env
- **Logs**: /var/log/pagepouch/
- **Service**: pagepouch.service (systemd)

### Initial Server Setup

#### 1. Create Application Directory and User Setup

```bash
# On production server as root
sudo mkdir -p /opt/pagepouch
sudo chown blog:blog /opt/pagepouch
```

#### 2. Create Environment Configuration

```bash
# Create /opt/pagepouch/.env as blog user
sudo -u blog tee /opt/pagepouch/.env << 'EOF'
DATABASE_URL=sqlite:///opt/pagepouch/pagepouch.db
PAGEPOUCH_KEY_BASE_64=your_generated_key_here
RUST_LOG=info
EOF
```

#### 3. Create Systemd Service

```bash
# Create /etc/systemd/system/pagepouch.service
sudo tee /etc/systemd/system/pagepouch.service << 'EOF'
[Unit]
Description=PagePouch Bookmark Manager
After=network.target

[Service]
Type=simple
User=blog
Group=blog
WorkingDirectory=/opt/pagepouch
ExecStart=/opt/pagepouch/pagepouch
EnvironmentFile=/opt/pagepouch/.env
Restart=always
RestartSec=10

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/pagepouch

[Install]
WantedBy=multi-user.target
EOF

# Enable and start the service
sudo systemctl daemon-reload
sudo systemctl enable pagepouch
```

#### 5. Configure Nginx

```bash
# Create /etc/nginx/sites-available/pagepouch.com
sudo tee /etc/nginx/sites-available/pagepouch.com << 'EOF'
server {
    listen 80;
    server_name pagepouch.com www.pagepouch.com;

    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name pagepouch.com www.pagepouch.com;

    # SSL certificates (will be managed by certbot)
    ssl_certificate /etc/letsencrypt/live/pagepouch.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/pagepouch.com/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;

    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # Proxy to PagePouch
    location / {
        proxy_pass http://127.0.0.1:8888;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket support if needed
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # Health check endpoint for uptime-kuma
    location /health {
        access_log off;
        proxy_pass http://127.0.0.1:8888;
    }
}
EOF

# Enable the site
sudo ln -sf /etc/nginx/sites-available/pagepouch.com /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
```

#### 6. Setup SSL Certificate

```bash
# Generate SSL certificate with certbot
sudo certbot --nginx -d pagepouch.com -d www.pagepouch.com
```

### Webhook Configuration

#### 1. Create Webhook Configuration

```bash
# Add to your existing webhook configuration
# Path: /path/to/webhook/hooks.json (add to existing array)
{
  "id": "pagepouch-deploy",
  "execute-command": "/opt/pagepouch/deploy.sh",
  "command-working-directory": "/opt/pagepouch",
  "response-message": "PagePouch deployment initiated",
  "pass-arguments-to-command": [
    {
      "source": "payload",
      "name": "release.tag_name"
    },
    {
      "source": "payload",
      "name": "release.assets.0.browser_download_url"
    }
  ],
  "trigger-rule": {
    "and": [
      {
        "match": {
          "type": "payload-hash-sha256",
          "secret": "your-webhook-secret",
          "parameter": {
            "source": "header",
            "name": "X-Hub-Signature-256"
          }
        }
      },
      {
        "match": {
          "type": "value",
          "value": "released",
          "parameter": {
            "source": "payload",
            "name": "action"
          }
        }
      },
      {
        "match": {
          "type": "value",
          "value": "pagepouch-rs",
          "parameter": {
            "source": "payload",
            "name": "repository.name"
          }
        }
      }
    ]
  }
}
```

#### 2. Setup sudo permissions for blog user

```bash
# Create sudoers file for PagePouch service management
sudo tee /etc/sudoers.d/pagepouch << 'EOF'
blog ALL=(ALL) NOPASSWD: /bin/systemctl start pagepouch, /bin/systemctl stop pagepouch, /bin/systemctl restart pagepouch, /bin/systemctl status pagepouch
EOF
```

#### 3. Setup Telegram notifications (optional)

If you want deployment notifications via Telegram:

```bash
# Add to blog user's environment (e.g., in /home/blog/.bashrc or /home/blog/.profile)
export TELEGRAM_BOT_TOKEN="your_bot_token_here"
export TELEGRAM_CHAT_ID="your_chat_id_here"
```

#### 4. Create Deployment Script

```bash
# Create /opt/pagepouch/deploy.sh
sudo tee /opt/pagepouch/deploy.sh << 'EOF'
#!/bin/bash
set -euo pipefail

TAG_NAME="$1"
DOWNLOAD_URL="$2"
BINARY_PATH="/opt/pagepouch/pagepouch"
BACKUP_PATH="/opt/pagepouch/pagepouch.backup"
TEMP_PATH="/tmp/pagepouch-${TAG_NAME}"

echo "🚀 Starting PagePouch deployment for version ${TAG_NAME}"

# Create backup of current binary
if [[ -f "$BINARY_PATH" ]]; then
    echo "📦 Backing up current binary"
    cp "$BINARY_PATH" "$BACKUP_PATH"
fi

# Download new binary
echo "⬇️  Downloading new binary from ${DOWNLOAD_URL}"
curl -L -o "$TEMP_PATH" "$DOWNLOAD_URL"

# Make executable and move to final location
chmod +x "$TEMP_PATH"
mv "$TEMP_PATH" "$BINARY_PATH"

# Database migrations will run automatically when the service starts

# Restart service (using sudo since pagepouch runs as system service)
echo "🔄 Restarting PagePouch service"
sudo systemctl restart pagepouch

# Wait a moment and check if service is running
sleep 3
if sudo systemctl is-active --quiet pagepouch; then
    echo "✅ PagePouch deployment successful for version ${TAG_NAME}"

    # Send Telegram notification (optional)
    if [[ -n "${TELEGRAM_BOT_TOKEN:-}" ]] && [[ -n "${TELEGRAM_CHAT_ID:-}" ]]; then
        curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
            -d "chat_id=${TELEGRAM_CHAT_ID}" \
            -d "text=✅ PagePouch ${TAG_NAME} deployed successfully to https://pagepouch.com" \
            -d "parse_mode=HTML" > /dev/null
    fi
else
    echo "❌ PagePouch service failed to start"

    # Restore backup if available
    if [[ -f "$BACKUP_PATH" ]]; then
        echo "🔙 Restoring backup"
        mv "$BACKUP_PATH" "$BINARY_PATH"
        sudo systemctl restart pagepouch
    fi

    # Send failure notification
    if [[ -n "${TELEGRAM_BOT_TOKEN:-}" ]] && [[ -n "${TELEGRAM_CHAT_ID:-}" ]]; then
        curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
            -d "chat_id=${TELEGRAM_CHAT_ID}" \
            -d "text=❌ PagePouch ${TAG_NAME} deployment failed. Service restored to previous version." \
            -d "parse_mode=HTML" > /dev/null
    fi

    exit 1
fi
EOF

# Make executable
sudo chmod +x /opt/pagepouch/deploy.sh
sudo chown blog:blog /opt/pagepouch/deploy.sh
```

### Release Process

#### 1. Automated Release Script

An enhanced release script is available at `scripts/release.sh` that automatically handles the entire build and release process.

**Features:**

- Validates version consistency between `Cargo.toml` and `CHANGELOG.md`
- Extracts release notes automatically from `CHANGELOG.md`
- Checks for existing tags both locally and remotely
- Validates changelog content has proper structure
- Builds, tests, and creates GitHub releases

**Prerequisites:**

- GitHub CLI (`gh`) installed and authenticated
- Clean git working directory
- All tests passing
- Version in `Cargo.toml` matches the most recent release in `CHANGELOG.md`
- CHANGELOG.md contains proper release notes with `### headings` and `- bullet points`

#### 2. Usage

Simply run the script without arguments:

```bash
./scripts/release.sh
```

**The script will automatically:**

1. Extract version from `Cargo.toml`
2. Validate it matches the most recent release in `CHANGELOG.md`
3. Extract release notes from `CHANGELOG.md`
4. Check for existing tags locally and remotely
5. Run tests and clippy
6. Build optimized binary
7. Create and push git tag
8. Create GitHub release with binary and changelog notes
9. Check deployment health after 30 seconds

**Before releasing, ensure:**

- Update version in `Cargo.toml`
- Add a new release section in `CHANGELOG.md` with the same version
- Move content from `## [Unreleased]` section to the new release section
- Include at least one `###` subsection with `- bullet points`

#### 2. GitHub Webhook Setup

1. Go to your GitHub repository settings
2. Add webhook: https://ansonvandoren.com/hooks/pagepouch-deploy
3. Content type: application/json
4. Secret: (set your webhook secret)
5. Events: Releases only

### Monitoring Setup

#### 1. Uptime-Kuma Configuration

- Add new monitor for https://pagepouch.com/health
- Set appropriate intervals and notifications

#### 2. Service Management Commands

```bash
# Check service status
sudo systemctl status pagepouch

# View logs
sudo journalctl -u pagepouch -f
tail -f /var/log/pagepouch/stdout.log
tail -f /var/log/pagepouch/stderr.log

# Restart service
sudo systemctl restart pagepouch

# View nginx logs
sudo tail -f /var/log/nginx/access.log
sudo tail -f /var/log/nginx/error.log
```

### Troubleshooting

#### Common Issues

1. **Service won't start**: Check logs with `journalctl -u pagepouch`
2. **Database issues**: Ensure blog user has write access to `/opt/pagepouch/`
3. **SSL certificate issues**: Run `sudo certbot renew --dry-run`
4. **Port conflicts**: Check with `sudo netstat -tlnp | grep 8888`

#### Emergency Rollback

```bash
# If deployment fails and backup is available
sudo systemctl stop pagepouch
sudo mv /opt/pagepouch/pagepouch.backup /opt/pagepouch/pagepouch
sudo systemctl start pagepouch
```
