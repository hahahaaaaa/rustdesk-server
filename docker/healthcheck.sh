#!/bin/sh

/package/admin/s6/command/s6-svstat /run/s6-rc/servicedirs/hbbr || exit 1
/package/admin/s6/command/s6-svstat /run/s6-rc/servicedirs/hbbs || exit 1
/package/admin/s6/command/s6-svstat /run/s6-rc/servicedirs/api || exit 1
wget -q -O /dev/null "http://127.0.0.1:${API_PORT:-21114}/api/health" || exit 1
