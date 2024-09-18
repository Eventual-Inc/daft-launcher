import daft

print('Hello, world!')
try:
    raise Exception('This is an error!')
except:
    print('Exception caught!')
