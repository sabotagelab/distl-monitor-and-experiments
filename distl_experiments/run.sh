#!/usr/bin/env fish

# --- Configuration ---
# --- Function: replace a specific line in a file ---
function replace_line
    set file $argv[1]
    set line_number $argv[2]
    set new_text $argv[3]

    set tmp (mktemp)

    awk -v n=$line_number -v s="$new_text" '
        NR == n { print s; next }
        { print }
    ' $file > $tmp

    mv $tmp $file
end

function run_n
    set fmla $argv[1]
    set agents $argv[2]

    set MAIN_PATH "./src/main.rs"
    set DISTL_PATH "../distl/src/lib.rs"
    set MAIN_LINE_NUM 97
    set FMLA_LINE_NUM 115
    set DISTL_LINE_NUM 8
    set EPS_LINE_NUM 9
    set EPS_CONTENT "const EPS: f64 = 0.05;"

    set FMLA_CONTENT "        let formula = _$fmla(N);"
    set CSV_OUTPUT "results/$fmla-results.csv"

    replace_line $MAIN_PATH $FMLA_LINE_NUM $FMLA_CONTENT

    for i in (seq 2 $agents)
        set MAIN_CONTENT "    const N: usize = $i;"
        set DISTL_CONTENT "const N: usize = $i;"

        replace_line $MAIN_PATH $MAIN_LINE_NUM $MAIN_CONTENT
        replace_line $DISTL_PATH $DISTL_LINE_NUM $DISTL_CONTENT

        # --- Run a cargo command ---
        cargo run -q --release >> $CSV_OUTPUT
    end
    echo "Results written to $CSV_OUTPUT"
end

# run_n "f2" 8
# run_n "f3" 11
# run_n "f1" 150
run_n "f2" 4
run_n "f3" 6
run_n "f1" 30
