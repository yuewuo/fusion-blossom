set terminal postscript eps color "Arial, 24"
set terminal postscript landscape
set xlabel "Partition Count" font "Arial, 24"
set x2label "Partition Size" font "Arial, 24"
set ylabel "Decoding Time Per Syndrome (us)" font "Arial, 24"
set grid ytics
set size 1,1

set logscale x
set logscale x2
set xrange [1:7500]
set x2range [1:7500]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '3' 3, '10' 10, '30' 30, '100' 100, '300' 300, '1000' 1000, '3000' 3000)
set x2tics ('1e5' 1, '3.3e4' 3, '1e4' 10, '3.3e3' 30, '1e3' 100, '3.3e2' 300, '100' 1000, '33' 3000)
# set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('2' 2e-6, '3' 3e-6, '4' 4e-6)
set yrange [2e-6:4e-6]
set key outside horizontal top center font "Arial, 24"
set style fill transparent solid 0.2 noborder
set key samplen 4

set arrow from 2000, graph 0 to 2000, graph 1 nohead lc rgb "blue"

set output "decoding_time_per_syndrome.eps"

plot "data.txt" using 1:4 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype 7 pointsize 1.3 notitle

system("ps2pdf -dEPSCrop decoding_time_per_syndrome.eps decoding_time_per_syndrome.pdf")
