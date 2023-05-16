load "../paper_settings.gp"
set terminal postscript eps color default_font
set terminal postscript landscape
set xlabel "Threads" font default_font
set ylabel "Decoding time per round ({/Symbol m}s)" font default_font
set title "p = 0.5%, 10^{5} {/Symbol \264} 21 {/Symbol \264} 21"
set grid ytics
set size 1,1

set style line 12 lc rgb '0xCCCCCC' lt 1 lw 2
set grid ytics ls 12
set grid xtics ls 12

set lmargin 5
set rmargin 0
set tmargin 1
set bmargin 1

set logscale x
set xrange [1:256]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '2' 2, '4' 4, '8' 8, '16' 16, '32' 32, '64' 64, '128' 128, '256' 256)
set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('0.05' 5e-8, '0.1' 1e-7, '0.3' 3e-7, '1' 1e-6, '3' 3e-6, '10' 1e-5, '30' 3e-5)
set yrange [4e-7:7e-5]
set style fill transparent solid 0.2 noborder
set key box top right Left reverse width -2.8 height 0.5 opaque font default_font samplen 2

set output "pymatching_compare.eps"

pymatching_per_round = "`head -2 data.txt | tail -1 | awk '{print $3}'`"

plot "../thread_pool_size_partition_2k/data.txt" using 1:3 with linespoints lt rgb "#9400D3" linewidth 3 pointtype point_type_fusion pointsize default_point_size title "Fusion Blossom",\
    1.1377104518828523e-07 + 3.864736116753137e-05/x with lines dashtype 2 lt rgb "#9400D3" linewidth 3 title "(linear speedup)",\
    pymatching_per_round + 0 * x with lines lt rgb "#279627" linewidth 3 notitle,\
    "data.txt" using 1:3 with linespoints lt rgb "#279627" linewidth 3 pointtype point_type_sparse pointsize default_point_size title "Sparse Blossom"

system("ps2pdf -dEPSCrop pymatching_compare.eps pymatching_compare.pdf")

# system("pdf2svg pymatching_compare.pdf pymatching_compare.svg")
