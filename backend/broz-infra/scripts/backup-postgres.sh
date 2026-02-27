#!/usr/bin/env bash
# ============================================================================
# BROZ — PostgreSQL Backup Script
# ============================================================================
# Performs pg_dump of all 7 databases with retention policy:
#   - 7 daily backups
#   - 4 weekly backups (Sundays)
#   - 3 monthly backups (1st of month)
#
# Storage targets:
#   - Local: /var/backups/broz/postgres/
#   - Remote: Hetzner Object Storage (S3-compatible) via aws cli
#
# Usage:
#   ./backup-postgres.sh                    # Run from App Node
#   ./backup-postgres.sh --upload           # Run + upload to Object Storage
#
# Cron (daily at 03:00):
#   0 3 * * * /root/broz-infra/scripts/backup-postgres.sh --upload >> /var/log/broz-backup.log 2>&1
# ============================================================================

set -euo pipefail

UPLOAD=false
[ "${1:-}" = "--upload" ] && UPLOAD=true

BACKUP_DIR="/var/backups/broz/postgres"
DATABASES="broz_auth broz_user broz_matching broz_messaging broz_notification broz_moderation broz_analytics"
DATE=$(date +%Y-%m-%d)
DAY_OF_WEEK=$(date +%u)  # 1=Monday, 7=Sunday
DAY_OF_MONTH=$(date +%d)
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Retention
DAILY_KEEP=7
WEEKLY_KEEP=4
MONTHLY_KEEP=3

# S3 config (Hetzner Object Storage)
S3_BUCKET="${S3_BUCKET:-s3://broz-backups}"
S3_ENDPOINT="${S3_ENDPOINT:-https://fsn1.your-objectstorage.com}"

# PostgreSQL connection (from K3s pod or external)
PG_HOST="${PG_HOST:-postgres.broz-data.svc.cluster.local}"
PG_USER="${PG_USER:-brozadmin}"
PG_PASSWORD="${PG_PASSWORD:-}"

echo "================================================"
echo "  BROZ PostgreSQL Backup — $DATE"
echo "================================================"

mkdir -p "$BACKUP_DIR"/{daily,weekly,monthly}

# ── Determine backup type ──────────────────────────────────────────────────

BACKUP_TYPE="daily"
if [ "$DAY_OF_MONTH" = "01" ]; then
    BACKUP_TYPE="monthly"
elif [ "$DAY_OF_WEEK" = "7" ]; then
    BACKUP_TYPE="weekly"
fi
echo "  Backup type: $BACKUP_TYPE"

# ── Dump each database ────────────────────────────────────────────────────

BACKUP_FILE="$BACKUP_DIR/$BACKUP_TYPE/broz_all_${TIMESTAMP}.sql.gz"
echo "  Output: $BACKUP_FILE"

# Check if running inside K3s or externally
if kubectl get pods -n broz-data &>/dev/null; then
    # Running from the App Node — exec into postgres pod
    PG_POD=$(kubectl get pod -l app=postgres -n broz-data -o jsonpath='{.items[0].metadata.name}')

    echo "  Dumping via K3s pod: $PG_POD"
    {
        for db in $DATABASES; do
            echo "  Dumping $db..."
            kubectl exec "$PG_POD" -n broz-data -- \
                pg_dump -U "$PG_USER" --format=plain --no-owner --no-acl "$db"
            echo ""
            echo "-- END DATABASE: $db"
            echo ""
        done
    } | gzip > "$BACKUP_FILE"
else
    # Running externally with direct PG access
    echo "  Dumping via direct connection: $PG_HOST"
    {
        for db in $DATABASES; do
            echo "  Dumping $db..."
            PGPASSWORD="$PG_PASSWORD" pg_dump -h "$PG_HOST" -U "$PG_USER" \
                --format=plain --no-owner --no-acl "$db"
            echo ""
            echo "-- END DATABASE: $db"
            echo ""
        done
    } | gzip > "$BACKUP_FILE"
fi

BACKUP_SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
echo "  Backup size: $BACKUP_SIZE"

# ── Verify backup ─────────────────────────────────────────────────────────

if ! gzip -t "$BACKUP_FILE" 2>/dev/null; then
    echo "  ERROR: Backup file is corrupted!"
    exit 1
fi
echo "  Backup verified (gzip integrity OK)."

# ── Upload to Object Storage ──────────────────────────────────────────────

if [ "$UPLOAD" = true ]; then
    if command -v aws &>/dev/null; then
        echo "  Uploading to $S3_BUCKET..."
        aws s3 cp "$BACKUP_FILE" \
            "$S3_BUCKET/postgres/$BACKUP_TYPE/$(basename "$BACKUP_FILE")" \
            --endpoint-url "$S3_ENDPOINT"
        echo "  Upload complete."
    else
        echo "  WARNING: aws cli not installed — skipping upload."
    fi
fi

# ── Retention cleanup ─────────────────────────────────────────────────────

echo "  Applying retention policy..."

cleanup_old_backups() {
    local dir=$1
    local keep=$2
    local count
    count=$(find "$dir" -name "broz_all_*.sql.gz" -type f | wc -l)
    if [ "$count" -gt "$keep" ]; then
        local to_delete=$((count - keep))
        find "$dir" -name "broz_all_*.sql.gz" -type f -printf '%T@ %p\n' | \
            sort -n | head -n "$to_delete" | cut -d' ' -f2- | \
            while read -r f; do
                echo "    Deleting old backup: $(basename "$f")"
                rm -f "$f"
            done
    fi
}

cleanup_old_backups "$BACKUP_DIR/daily" "$DAILY_KEEP"
cleanup_old_backups "$BACKUP_DIR/weekly" "$WEEKLY_KEEP"
cleanup_old_backups "$BACKUP_DIR/monthly" "$MONTHLY_KEEP"

echo ""
echo "  Backup complete: $BACKUP_FILE ($BACKUP_SIZE)"
echo "================================================"
