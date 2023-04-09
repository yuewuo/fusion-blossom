load "../paper_settings.gp"
set terminal postscript eps color default_font
set terminal postscript landscape enhanced
set xlabel "Code Distance d" font default_font
set ylabel "Fusion Time ({/Symbol m}s)" font default_font
set title "p = 0.5%, 100 {/Symbol \264} d {/Symbol \264} d"
set size 1,1

set style line 12 lc rgb '0xCCCCCC' lt 1 lw 2
set grid ytics ls 12
set grid xtics ls 12

set lmargin 5
set rmargin 0
set tmargin 1
set bmargin 1

set logscale x
set xrange [2:700]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('3' 3, '5' 5, '7' 7, '10' 10, '20' 20, '50' 50, '100' 100)
set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('0.1' 1e-7, '1' 1e-6, '10' 10e-6, '100' 100e-6, '1000' 1000e-6)
set yrange [5e-8:2e-2]
set style fill transparent solid 0.2 noborder
set key box top left Left reverse width -3.5 height 0.5 opaque font default_font samplen 2

set output "fusion_time_d.eps"

plot "data.txt" using 1:3 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype point_type_parity pointsize default_point_size title ""

system("ps2pdf -dEPSCrop fusion_time_d.eps fusion_time_d.pdf")

# system("pdf2svg fusion_time_d.pdf fusion_time_d.svg")
