#!/usr/bin/env bash

# env
source env.sh

# stop all abci nodes
abcis=`pgrep -f abci_validator_node`
if ! [ -z "$abcis" ]
then
    echo -n "killed abci: "
    for pid in $abcis
    do
        kill -9 $pid
        echo -en "$pid "
    done
    echo
fi

# stop all tendermint nodes
nodes=`pgrep -f "tendermint node.*"`
if ! [ -z "$abcis" ]
then
    echo -n "killed node: "
    for pid in $nodes
    do
        kill -9 $pid
        echo -en "$pid "
    done
    echo
fi

exit 0
