set terminal postscript eps color "Arial, 24"
set terminal postscript landscape
set xlabel "Partition" font "Arial, 24"
set ylabel "Avergae Decoding time Per Syndrome (us)" font "Arial, 24"
set grid ytics
set size 1,1

set logscale x
set xrange [1:512]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '2' 2, '4' 4, '8' 8, '16' 16, '32' 32, '64' 64, '128' 128, '256' 256, '512' 512)
set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('1' 1e-6, '3' 3e-6, '10' 1e-5)
set yrange [1e-6:1.7e-5]
set key outside horizontal top center font "Arial, 24"

set style fill transparent solid 0.2 noborder
set key samplen 4
# set key maxrows 2
# set key height 5

set output "average_decoding_time_per_syndrome.eps"

plot 1.54e-5 / x notitle with lines dashtype 2 lt rgb "#e41a1c" linewidth 3,\
    "data.txt" using 1:4 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype 7 pointsize 1.3 notitle

system("ps2pdf -dEPSCrop average_decoding_time_per_syndrome.eps average_decoding_time_per_syndrome.pdf")

# set size 1,0.75
# set output "average_decoding_time_per_syndrome_w.eps"
# replot
# system("ps2pdf -dEPSCrop average_decoding_time_per_syndrome_w.eps average_decoding_time_per_syndrome_w.pdf")

# set size 1,0.6
# set output "average_decoding_time_per_syndrome_w_w.eps"
# replot
# system("ps2pdf -dEPSCrop average_decoding_time_per_syndrome_w_w.eps average_decoding_time_per_syndrome_w_w.pdf")
