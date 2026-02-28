try:
    x = 1 / 0
except ZeroDivisionError:
    print('caught division by zero')
finally:
    print('finally block')
