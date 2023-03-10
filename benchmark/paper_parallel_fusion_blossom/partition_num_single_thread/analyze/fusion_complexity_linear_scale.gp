set terminal postscript eps color "Arial, 24"
set terminal postscript landscape
set xlabel "Fusion Index" font "Arial, 24"
set ylabel "Fusion Operation Time (us)" font "Arial, 24"
set grid ytics
set size 1,1

# set logscale x
set xrange [1:1000]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '500' 500, '750' 750, '875' 875, '1000' 1000)
# set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('40' 4e-5, '60' 6e-5, '80' 8e-5, '100' 1e-4, '120' 1.2e-4)
set yrange [3e-5:1.3e-4]
set key outside horizontal top center font "Arial, 24"
set style fill transparent solid 0.2 noborder
set key samplen 4

set output "fusion_complexity_linear_scale.eps"

plot "fusion_complexity.txt" using 1:2 with points lt rgb "#e41a1c" linewidth 3 pointtype 7 pointsize 0.5 notitle

system("ps2pdf -dEPSCrop fusion_complexity_linear_scale.eps fusion_complexity_linear_scale.pdf")
