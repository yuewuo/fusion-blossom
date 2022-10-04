set terminal postscript eps color "Arial, 24"
set terminal postscript landscape
set xlabel "Fusion Index" font "Arial, 24"
set ylabel "Fusion Complexity (us)" font "Arial, 24"
set grid ytics
set size 1,1

set logscale x
set xrange [1:1000]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '2' 2, '4' 4, '8' 8, '16' 16, '32' 32, '64' 64, '128' 128, '256' 256, '512' 512)
set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('10' 1e-5, '30' 3e-5, '100' 1e-4, '300' 3e-4, '1000' 1e-3)
set yrange [1e-5:1.2e-3]
set key outside horizontal top center font "Arial, 24"
set style fill transparent solid 0.2 noborder
set key samplen 4

set output "fusion_complexity.eps"

plot "fusion_complexity.txt" using 1:2 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype 7 pointsize 1.3 notitle

system("ps2pdf -dEPSCrop fusion_complexity.eps fusion_complexity.pdf")
