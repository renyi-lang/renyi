"""The same trial-division prime count as primes.ry, for CPython."""


def is_prime(candidate):
    if candidate < 2:
        return False
    divisor = 2
    while divisor * divisor <= candidate:
        if candidate % divisor == 0:
            return False
        divisor += 1
    return True


found = 0
for candidate in range(2, 200001):
    if is_prime(candidate):
        found += 1
print(found)
