#!/bin/sh
# Invoked as data by /bin/sh: no user's login files or environment are read.
case "$VESSEL_SHELL_FIXTURE" in
  fallback)
    if [ "$1" = -i ]; then exit 7; fi
    ;;
  overflow)
    exec /usr/bin/head -c 1048578 /dev/zero
    ;;
  hang)
    exec /bin/sleep 60
    ;;
  *) exit 2 ;;
esac
for argument do command=$argument; done
printf 'fixture profile noise\n'
exec /bin/sh -c "$command"
