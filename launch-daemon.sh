#!/bin/bash
if [[ ! $(pgrep cartel) ]]; then
    (cartel-daemon >/dev/null &)
fi

