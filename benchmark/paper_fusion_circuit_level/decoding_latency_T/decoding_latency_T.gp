load "../paper_settings.gp"
set terminal postscript eps color default_font
set terminal postscript landscape enhanced
set xlabel "Code Distance d" font default_font
set ylabel "Latency (ms)" font default_font
set title "10^5 {/Symbol \264} d {/Symbol \264} d"
set size 1,1

set style line 12 lc rgb '0xCCCCCC' lt 1 lw 2
set grid ytics ls 12
set grid xtics ls 12

set lmargin 5
set rmargin 0
set tmargin 1
set bmargin 1

set logscale x
set xrange [10:1e5]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
# set xtics ('11' 11, '13' 13, '15' 15, '17' 17, '21' 21, '25' 25, '31' 31, '35' 35)
set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('1' 1e-3, '10' 1e-2, '100' 1e-1)
set yrange [1e-4:2e-1]
set style fill transparent solid 0.2 noborder
set key box top left Left reverse width -3.5 height 0.5 opaque font default_font samplen 2

set output "decoding_latency_T.eps"

plot "data_batch.txt" using 1:3 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype point_type_parity pointsize default_point_size title "batch",\
    "data_stream.txt" using 1:3 with linespoints lt rgb "blue" linewidth 3 pointtype point_type_parity pointsize default_point_size title "stream"

system("ps2pdf -dEPSCrop decoding_latency_T.eps decoding_latency_T.pdf")

# system("pdf2svg decoding_latency_T.pdf decoding_latency_T.svg")
