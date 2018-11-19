#!/usr/bin/env bash

set -e;
export PROJECT_ROOT=$(git rev-parse --show-toplevel)
cd "$PROJECT_ROOT/api_tests";

source "$PROJECT_ROOT/api_tests/harness-lib.sh"

END(){
    set +e;
    for pid in "$BOB_COMIT_NODE_PID" "$ALICE_COMIT_NODE_PID" "$LQS_PID" "$BTC_BLOCKLOOP_PID"; do
        if test "$pid" && ps "$pid" >/dev/null; then
            echo "KILLING $pid";
            kill "$pid" 2>/dev/null;
            # Here if one of the above is a job is doesn't print out an annoying "Terminated" line to stderr
            wait "$pid" 2>/dev/null;
        fi
    done
    log "KILLING docker containers";
    (
        cd regtest;
        docker-compose rm -sfv bitcoin ethereum;
    );
}

trap 'END' EXIT;

function setup() {
    if test "$LOG_DIR"; then
        mkdir -p "$LOG_DIR"
        rm -f "$LOG_DIR/*.log"
    fi

    #### Env variable to run all services
    set -a;
    source ./regtest/regtest.env
    set +a;

    export BITCOIN_RPC_URL="http://$BITCOIN_RPC_HOST:$BITCOIN_RPC_PORT";
    #### Start all services
    (
        cd ./regtest;
        log "Starting up docker containers"
        docker-compose up -d bitcoin ethereum
        if test -d "$LOG_DIR"; then
            log_file="$LOG_DIR/docker-compose.log"
            docker-compose logs --tail=all >$log_file
        fi
    );

    sleep 10;

    npm test ./e2e/rfc003/btc_erc20/setup.js

    export BOB_CONFIG_FILE=./regtest/bob/default.toml;
    BOB_COMIT_NODE_PID=$(
        export RUST_BACKTRACE=1 \
               COMIT_NODE_CONFIG_PATH=./regtest/bob;
        start_target "comit_node" "Bob";
    );

    export ALICE_COMIT_NODE_HOST=127.0.0.1;
    export ALICE_CONFIG_FILE=./regtest/alice/default.toml;
    ALICE_COMIT_NODE_PID=$(
        export COMIT_NODE_CONFIG_PATH=./regtest/alice;
        start_target "comit_node" "Alice";
    );

    LQS_PID=$(
        export LEDGER_QUERY_SERVICE_CONFIG_PATH=./regtest/ledger_query_service
        export ETHEREUM_POLLING_TIME_SEC=1
        export RUST_LOG=debug;

        start_target "ledger_query_service" "LQS";
    );
}

test "$*" || { log "ERROR: The harness requires to test to run!"; exit 1; }

setup;

debug "Bitcoin RPC url: $BITCOIN_RPC_URL";
debug "Ethereum node url: $ETHEREUM_NODE_ENDPOINT";
activate_segwit;

fund_bitcoin_address;
generate_btc_blocks_every 5;
sleep 2;

npm test "$@";
