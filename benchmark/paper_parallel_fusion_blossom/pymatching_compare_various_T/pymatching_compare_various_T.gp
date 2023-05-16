load "../paper_settings.gp"
set terminal postscript eps color default_font
set terminal postscript landscape
set xlabel "Measurement Rounds T" font default_font
set ylabel "Decoding time per round ({/Symbol m}s)" font default_font
set title "p = 0.5%, T {/Symbol \264} 21 {/Symbol \264} 21"
set size 1,1

set style line 12 lc rgb '0xCCCCCC' lt 1 lw 2
set grid ytics ls 12
set grid xtics ls 12

set lmargin 5
set rmargin 0
set tmargin 1
set bmargin 1

set logscale x
set xrange [1:100000]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '10' 10, '10^2' 100, '10^3' 1000, '10^4' 10000, '10^5' 100000)
set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('0.3' 3e-7, '1' 1e-6, '3' 3e-6, '10' 1e-5, '30' 3e-5)
set yrange [2e-6:10e-5]
set style fill transparent solid 0.2 noborder
set key box top left Left reverse width -0.5 height 0.5 opaque font default_font samplen 2


set output "pymatching_compare_various_T.eps"

plot "data_fusion.txt" using 1:3 with linespoints lt rgb "#9400D3" linewidth 3 pointtype point_type_fusion pointsize default_point_size title "Fusion Blossom",\
    "" using 1:3:($3*(1-$5)):($3*(1+$5)) with errorbars lt rgb "#9400D3" linewidth 4 pointtype point_type_fusion pointsize default_point_size notitle,\
    "data_parity.txt" using 1:3 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype point_type_parity pointsize default_point_size title "Parity Blossom",\
    "" using 1:3:($3*(1-$5)):($3*(1+$5)) with errorbars lt rgb "#e41a1c" linewidth 4 pointtype point_type_parity pointsize default_point_size notitle,\
    "data_pymatching.txt" using 1:3 with linespoints lt rgb "#279627" linewidth 3 pointtype point_type_sparse pointsize default_point_size title "Sparse Blossom"

system("ps2pdf -dEPSCrop pymatching_compare_various_T.eps pymatching_compare_various_T.pdf")

system("pdf2svg pymatching_compare_various_T.pdf pymatching_compare_various_T.svg")
