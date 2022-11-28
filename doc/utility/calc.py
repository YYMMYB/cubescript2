# %% 计算视野内chunk数量
import math

class OctView:
    def __init__(self) -> None:
        pass

    def culc(self, r):
        def one_slice(n):
            if n == 0:
                return 1
            if n == 1:
                return 5
            a = n*(n-1)/2
            res = 4*n+4*a+1
            return res

        res = 0
        for n in range(r):
            s = one_slice(n)
            res += s * 2

        s = one_slice(r)
        res += s
        return res


octview = OctView()

num1 = octview.culc(12)
num1 = num1 * 16**3
print(num1)

num = octview.culc(24)
print(num)

i = 0
r = 24
ri = r
num2 = 0
while ri>=1:
    i+=1
    num2 += octview.culc(r)
    ri /= 2

print(i, num2)
num2 *= 16**3

print(num2)
print(num2/num1)

print(math.log2(num1))

# %%
