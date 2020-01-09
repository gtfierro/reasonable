#!/bin/bash

mv bench.txt bench.txt-old
export RUST_LOG=info
ontologies=$(ls example_models/ontologies)

# start with ontologies
REASONABLE_START=$(date +%s.%N)
./target/release/reasonable $ontologies
REASONABLE_END=$(date +%s.%N)
REASONABLE_DIFF=$(echo "$REASONABLE_END - $REASONABLE_START" | bc)

OWLRL_START=$(date +%s.%N)
./scripts/reason_owlrl.py $ontologies
OWLRL_END=$(date +%s.%N)
OWLRL_DIFF=$(echo "$OWLRL_END - $OWLRL_START" | bc)

SPEEDUP=$(echo "$OWLRL_DIFF / $REASONABLE_DIFF" | bc)

cat <<EOF >>bench.txt
brick+rdfs  $REASONABLE_DIFF  $OWLRL_DIFF  $SPEEDUP
EOF


for file in $(ls example_models/*.n3); do
    REASONABLE_START=$(date +%s.%N)
    ./target/release/reasonable $ontologies $file
    REASONABLE_END=$(date +%s.%N)
    REASONABLE_DIFF=$(echo "$REASONABLE_END - $REASONABLE_START" | bc)

    OWLRL_START=$(date +%s.%N)
    ./scripts/reason_owlrl.py $ontologies $file
    OWLRL_END=$(date +%s.%N)
    OWLRL_DIFF=$(echo "$OWLRL_END - $OWLRL_START" | bc)

    SPEEDUP=$(echo "$OWLRL_DIFF / $REASONABLE_DIFF" | bc)

    cat <<EOF >>bench.txt
$file  $REASONABLE_DIFF  $OWLRL_DIFF  $SPEEDUP
EOF
done
