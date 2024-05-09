from run import *


data_file = os.path.join(script_dir, f"distribution.txt")
with open(data_file, "w", encoding="utf8") as f:
    f.write(
        "# <d> <decoded> <add_defects> <primal> <dual> <simple_match> <complex_match> <potential speedup> <max potential>\n"
    )

    for d in d_vec:
        benchmark_profile_path = os.path.join(tmp_dir, f"generated_d{d}.profile")
        profile = Profile(benchmark_profile_path)
        decoded = profile.average_decoding_time()
        add_defects = profile.average_custom_time("add_defects")
        primal = profile.average_custom_time("primal")
        dual = profile.average_custom_time("dual")
        simple_match = profile.average_custom_time("simple_match")
        complex_match = profile.average_custom_time("complex_match")

        f.write(
            "%d %.5e %.5e %.5e %.5e %.5e %.5e %.3e %.3e \n"
            % (
                d,
                decoded,
                add_defects,
                primal,
                dual,
                simple_match,
                complex_match,
                decoded / primal,
                decoded / complex_match,
            )
        )
        f.flush()
