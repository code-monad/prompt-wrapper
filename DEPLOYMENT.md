# Deployment Guide

This document outlines how to deploy the Sayings API using Docker and Docker Compose.

## Prerequisites

- Docker and Docker Compose installed on your system
- OpenRouter API key

## Deployment Steps

1. **Clone the repository**

```bash
git clone <repository-url>
cd <repository-directory>
```

2. **Configure environment variables**

Copy the example environment file and edit it:

```bash
cp .env.example .env
```

Edit the `.env` file and add your OpenRouter API key and any other customizations.

3. **Build and start the services**

```bash
docker-compose up -d
```

This will:
- Build the Rust application
- Start the API service on port 3020
- Configure sled as the embedded database

4. **Verify deployment**

```bash
# Check running containers
docker-compose ps

# Check application logs
docker-compose logs -f app
```

5. **Access the API**

The API will be available at `http://localhost:3020`.

## Managing the Deployment

**Stopping the services:**
```bash
docker-compose down
```

**Restarting the services:**
```bash
docker-compose restart
```

**Updating to a new version:**
```bash
git pull
docker-compose build
docker-compose up -d
```

## Data Persistence

Sled database files are stored in a Docker volume named `sled_data` and will persist between container restarts.

## Backup and Recovery

To backup the sled database:

```bash
# Create a backup directory
mkdir -p backups

# Copy data from the Docker volume to your backup directory
docker run --rm -v prompt-wrapper_sled_data:/source -v $(pwd)/backups:/backup alpine cp -r /source/. /backup
```

To restore:

```bash
# Restore from backup to the Docker volume
docker run --rm -v $(pwd)/backups:/source -v prompt-wrapper_sled_data:/backup alpine cp -r /source/. /backup
```

## Scaling (Optional)

For production environments with higher load, consider:
- Using a managed database service
- Implementing a load balancer
- Setting up monitoring and alerting 