set terminal postscript eps color "Arial, 24"
set terminal postscript landscape
set xlabel "Noisy Measurement Rounds" font "Arial, 24"
set ylabel "Decoding time Per Defect (us)" font "Arial, 24"
set grid ytics
set size 1,1

set logscale x
set xrange [1:100000]
# print(", ".join([f"'{2 ** i}' {2 ** i}" for i in range(0,10)]))
set xtics ('1' 1, '10' 10, '10^2' 100, '10^3' 1000, '10^4' 10000, '10^5' 100000)
set logscale y
# print(", ".join([f"'1e{i}' 1e{i}" for i in range(-4, 2)]))
set ytics ('0.3' 3e-7, '1' 1e-6, '3' 3e-6, '10' 1e-5)
set yrange [2e-7:1.2e-5]
set key outside horizontal top center font "Arial, 24"
set style fill transparent solid 0.2 noborder
set key samplen 4

set output "pymatching_compare_various_T.eps"

plot "data_pymatching.txt" using 1:4 with linespoints lt rgb "blue" linewidth 3 pointtype 7 pointsize 1.3 title "PyMatching V2",\
    "data_fusion.txt" using 1:4 with linespoints lt rgb "#e41a1c" linewidth 3 pointtype 7 pointsize 1.3 title "Fusion Blossom"

system("ps2pdf -dEPSCrop pymatching_compare_various_T.eps pymatching_compare_various_T.pdf")
