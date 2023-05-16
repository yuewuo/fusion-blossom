

def nearest_odd(number):
    return round(number/2) * 2 + 1

d_vec = []

last_d = 1
for i in range(100):
    odd = nearest_odd(1 * (1.2 ** i))
    if odd == last_d:
        continue
    if odd > 100:
        break
    last_d = odd
    d_vec.append(odd)

print(d_vec)

# [3, 5, 7, 9, 11, 13, 17, 19, 23, 27, 33, 39, 47, 57, 67, 81, 97]
