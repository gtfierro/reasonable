#!/bin/bash

export RUST_LOG=info
ontologies=$(ls example_models/ontologies)
for file in $(ls example_models/*.n3); do
    ./target/release/reasonable $ontologies $file
    ./scripts/reason_owlrl.py $ontologies $file
done
