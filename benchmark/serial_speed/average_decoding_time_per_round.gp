set terminal postscript eps color "Arial, 24"
set terminal postscript landscape
set xlabel "Code Distance" font "Arial, 24"
set ylabel "Avergae Decoding time Per Measurement (us)" font "Arial, 24"
set grid ytics
set size 1,1

set logscale x
set xrange [3:81]
# print(", ".join([f"'{i}' {i}" for i in [3, 5, 7, 9, 15, 21, 27, 45, 63, 81]]))
set xtics ('3' 3, '5' 5, '7' 7, '9' 9, '15' 15, '21' 21, '27' 27, '45' 45, '63' 63, '81' 81)
set logscale y
# print(", ".join([f"'{int((10**i)*1000000)}' 1e{i}" for i in range(-7, -2)]))
set ytics ('0.1' 1e-7, '1' 1e-6, '10' 1e-5, '100' 1e-4, '1000' 1e-3)
set yrange [7e-7:3e-3]
set key outside horizontal top center font "Arial, 24"

set style fill transparent solid 0.2 noborder
set key samplen 4
# set key maxrows 2
# set key height 5

set output "average_decoding_time_per_round.eps"

plot 8.45e-7 * (x / 3) ** 2 notitle with lines dashtype 2 lt rgb "#e41a1c" linewidth 3,\
    "data.txt" using 1:3 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype 7 pointsize 1.3 notitle

system("ps2pdf -dEPSCrop average_decoding_time_per_round.eps average_decoding_time_per_round.pdf")

# set size 1,0.75
# set output "average_decoding_time_per_round_w.eps"
# replot
# system("ps2pdf -dEPSCrop average_decoding_time_per_round_w.eps average_decoding_time_per_round_w.pdf")

# set size 1,0.6
# set output "average_decoding_time_per_round_w_w.eps"
# replot
# system("ps2pdf -dEPSCrop average_decoding_time_per_round_w_w.eps average_decoding_time_per_round_w_w.pdf")
