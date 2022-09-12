set terminal postscript eps color "Arial, 24"
set terminal postscript landscape
set xlabel "Partition" font "Arial, 24"
set ylabel "Avergae Computation CPU Seconds (s)" font "Arial, 24"
set grid ytics
set size 1,1

set logscale x
set xrange [1:512]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '2' 2, '4' 4, '8' 8, '16' 16, '32' 32, '64' 64, '128' 128, '256' 256, '512' 512)
# print(", ".join([f"'{i}' {i}" for i in range(1, 16)]))
set ytics ('1' 1, '2' 2, '3' 3, '4' 4, '5' 5, '6' 6, '7' 7, '8' 8, '9' 9, '10' 10, '11' 11, '12' 12, '13' 13, '14' 14, '15' 15)
set yrange [0:15]
set key outside horizontal top center font "Arial, 24"

set style fill transparent solid 0.2 noborder
set key samplen 4
# set key maxrows 2
# set key height 5

set output "average_computation_cpu_seconds.eps"

plot 8.43 notitle with lines dashtype 2 lt rgb "#e41a1c" linewidth 3,\
    "data.txt" using 1:5 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype 7 pointsize 1.3 notitle

system("ps2pdf -dEPSCrop average_computation_cpu_seconds.eps average_computation_cpu_seconds.pdf")

# set size 1,0.75
# set output "average_computation_cpu_seconds_w.eps"
# replot
# system("ps2pdf -dEPSCrop average_computation_cpu_seconds_w.eps average_computation_cpu_seconds_w.pdf")

# set size 1,0.6
# set output "average_computation_cpu_seconds_w_w.eps"
# replot
# system("ps2pdf -dEPSCrop average_computation_cpu_seconds_w_w.eps average_computation_cpu_seconds_w_w.pdf")
