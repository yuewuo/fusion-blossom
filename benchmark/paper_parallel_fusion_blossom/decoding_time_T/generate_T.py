

def nearest_integer(number):
    return round(number)

T_vec = []

last_T = 0
for i in range(100):
    integer = nearest_integer(1 * (1.2 ** i))
    if integer == last_T:
        continue
    if integer > 1100:
        break
    last_T = integer
    T_vec.append(integer)

print(T_vec)

# [1, 2, 3, 4, 5, 6, 7, 9, 11, 13, 15, 18, 22, 27, 32, 38, 46, 55, 66, 79, 95, 114, 137, 165, 198, 237, 285, 342, 410, 492, 591, 709, 851, 1021]
