set terminal postscript eps color "Arial, 24"
set terminal postscript landscape
set xlabel "Maximum Tree Leaf Size" font "Arial, 24"
set ylabel "Decoding Time Per Syndrome (ns)" font "Arial, 24"
set grid ytics
set size 1,1

set logscale x
set xrange [1:7500]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '3' 3, '10' 10, '30' 30, '100' 100, '300' 300, '1000' 1000, '3000' 3000)
# set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('50' 5e-8, '100' 1e-7, '150' 1.5e-7, '200' 2e-7, '250' 2.5e-7, '300' 3e-7)
set yrange [5e-8:3e-7]
set key outside horizontal top center font "Arial, 24"
set style fill transparent solid 0.2 noborder
set key samplen 4

set arrow from 1000, graph 0 to 1000, graph 1 nohead lc rgb "blue"

set output "decoding_time_per_syndrome.eps"

plot "data.txt" using 1:4 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype 7 pointsize 1.3 notitle

system("ps2pdf -dEPSCrop decoding_time_per_syndrome.eps decoding_time_per_syndrome.pdf")
