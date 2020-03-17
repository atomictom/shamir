command="$1"

while true; do watch -g 'tree -D -p -h -a --timefmt="%s"' 2>&1 >/dev/null; RUST_BACKTRACE=1 cargo "${command}"; echo ------------------------------------; echo; echo; echo; sleep 1; done
