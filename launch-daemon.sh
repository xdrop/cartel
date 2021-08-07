#!/bin/bash
if [[ ! $(pgrep cartel-daemon) ]]; then
    cartel-daemon --detach >/dev/null
fi

