set terminal postscript eps color "Arial, 24"
set terminal postscript landscape
set xlabel "Threads" font "Arial, 24"
set ylabel "Decoding time Per Defect (us)" font "Arial, 24"
set grid ytics
set size 1,1

set logscale x
set xrange [1:64]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '2' 2, '4' 4, '8' 8, '16' 16, '32' 32, '64' 64)
set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('0.05' 5e-8, '0.1' 1e-7, '0.3' 3e-7, '1' 1e-6, '3' 3e-6)
set yrange [3e-8:5e-6]
set key outside horizontal top center font "Arial, 24"
set style fill transparent solid 0.2 noborder
set key samplen 4

set output "decoding_time_per_defect_trim.eps"

plot 3.42598e-06 / x notitle with lines dashtype 2 lt rgb "#e41a1c" linewidth 3,\
    "data.txt" using 1:4 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype 7 pointsize 1.3 notitle

system("ps2pdf -dEPSCrop decoding_time_per_defect_trim.eps decoding_time_per_defect_trim.pdf")

system("pdf2svg decoding_time_per_defect_trim.pdf decoding_time_per_defect_trim.svg")
