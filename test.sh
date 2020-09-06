command="$1"

while true; do watch -g 'tree -D -p -h -a --timefmt="%s"' >/dev/null 2>&1 ; RUST_BACKTRACE=1 cargo "${command}"; echo ------------------------------------; echo; echo; echo; sleep 1; done
