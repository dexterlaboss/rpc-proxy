#!/bin/bash

# Timeout settings
MAX_WAIT_TIME=60  # Maximum wait time in seconds
WAIT_INTERVAL=2   # Time to wait between attempts
TOTAL_WAIT=0      # Total time spent waiting

echo "Waiting for HBase to initialize..."
while ! echo "status" | /opt/hbase/bin/hbase shell &>/dev/null; do
  sleep $WAIT_INTERVAL
  TOTAL_WAIT=$((TOTAL_WAIT + WAIT_INTERVAL))
  if [ $TOTAL_WAIT -ge $MAX_WAIT_TIME ]; then
    echo "ERROR: HBase did not initialize within $MAX_WAIT_TIME seconds."
    exit 1
  fi
done
echo "HBase is ready."

# Define tables and column families
echo "Creating HBase tables..."
echo "create 'blocks', 'x'" | /opt/hbase/bin/hbase shell || echo "INFO: Table 'blocks' already exists, skipping."
echo "create 'tx', 'x'" | /opt/hbase/bin/hbase shell || echo "INFO: Table 'tx' already exists, skipping."
echo "create 'tx-by-addr', 'x'" | /opt/hbase/bin/hbase shell || echo "INFO: Table 'tx-by-addr' already exists, skipping."
echo "create 'tx_full', 'x'" | /opt/hbase/bin/hbase shell || echo "INFO: Table 'tx_full' already exists, skipping."

echo "HBase table creation completed."
exit 0