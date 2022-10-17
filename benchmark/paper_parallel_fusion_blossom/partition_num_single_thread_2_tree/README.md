
I'm using AWS m6i.4xlarge instance, because it needs roughly 32GB memory to run; price is 0.768USB/hour.

Up to 3.5 GHz 3rd generation Intel Xeon Scalable processors (Ice Lake 8375C)

Using a single-thread, I found the speed is 2.5x slower than my M1MAX CPU. Each syndrome takes 3us in the best case.

# update 2022.10.15

default feature is now: ["--features", "dangerous_pointer,u32_index,i32_weight"]
