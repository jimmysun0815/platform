#!/usr/bin/env bash

target_path=$1

if [[ ! -d "${target_path}/.git" ]]; then
    rm -rf $target_path
    url='https://gitee.com/findora-network/tendermint.git'
    if [[ 0 -eq `date | grep -c CST` ]]; then
        url='https://github.com/tendermint/tendermint.git'
    fi
    git clone -b v0.33.9 --depth=1 $url $target_path
fi
