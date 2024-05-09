from run import *

slurm_distribute.SLURM_DISTRIBUTE_CPUS_PER_TASK = 12


if __name__ == "__main__":

    @slurm_distribute.slurm_distribute_run(os.path.dirname(__file__))
    def experiment(
        slurm_commands_vec=None, run_command_get_stdout=run_command_get_stdout
    ):

        for p in p_vec:
            for d in d_vec:
                syndrome_file_path = os.path.join(
                    tmp_dir, f"generated-d{d}-p{p}.syndromes"
                )
                if os.path.exists(syndrome_file_path):
                    print(
                        "[warning] use existing syndrome data (if you think it's stale, delete it and rerun)"
                    )
                else:
                    command = fusion_blossom_qecp_generate_command(
                        d=d,
                        p=p,
                        total_rounds=total_rounds(d, p),
                        noisy_measurements=d - 1,
                    )
                    command += ["--code-type", "rotated-planar-code"]
                    command += ["--noise-model", "stim-noise-model"]
                    command += [
                        "--decoder",
                        "fusion",
                        "--decoder-config",
                        '{"only_stab_z":true,"use_combined_probability":true,"skip_decoding":true,"max_half_weight":7}',
                    ]
                    command += [
                        "--debug-print",
                        "fusion-blossom-syndrome-file",
                        "--fusion-blossom-syndrome-export-filename",
                        syndrome_file_path,
                    ]
                    command += ["--parallel", f"{STO(0)}"]  # use all cores
                    if slurm_commands_vec is not None:
                        slurm_commands_vec.sanity_checked_append(command)
                        continue
                    print(" ".join(command))

                    stdout, returncode = run_command_get_stdout(command)
                    print("\n" + stdout)
                    assert returncode == 0, "command fails..."
