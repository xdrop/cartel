#!/bin/bash
if [[ ! $(pgrep cartel-daemon) ]]; then
    (cartel-daemon >/dev/null &)
fi

