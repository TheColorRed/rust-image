import os
import tempfile
import numpy as np
import onnx
from onnx import helper, TensorProto
import sys, os
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
from lower_einsum_to_batched_matmul import lower_einsum
import onnxruntime as ort


def make_model(equation, a_shape, b_shape, out_shape):
    # create two inputs and a single Einsum node
    A = helper.make_tensor_value_info('A', TensorProto.FLOAT, list(a_shape))
    B = helper.make_tensor_value_info('B', TensorProto.FLOAT, list(b_shape))
    Y = helper.make_tensor_value_info('Y', TensorProto.FLOAT, list(out_shape))
    node = helper.make_node('Einsum', inputs=['A', 'B'], outputs=['Y'], name='einsum', equation=equation)
    graph = helper.make_graph([node], 'test', [A, B], [Y])
    model = helper.make_model(graph)
    return model


def run_inference(model, a_shape, b_shape):
    # write model to temp file
    fd, path = tempfile.mkstemp(suffix='.onnx')
    os.close(fd)
    onnx.save(model, path)
    sess = ort.InferenceSession(path, providers=['CPUExecutionProvider'])
    inp = sess.get_inputs()
    assert len(inp) == 2
    A = np.random.randn(*a_shape).astype('float32')
    B = np.random.randn(*b_shape).astype('float32')
    out = sess.run(None, {'A': A, 'B': B})
    os.remove(path)
    return out


def test_lower_einsum_pattern1():
    # hb w p c pattern: h=1,b=1,w=2,p=3,q=4,c=2
    eq = 'hbwpc,hbwqc->hbwpq'
    A_shape = (1,1,2,3,2)
    B_shape = (1,1,2,4,2)
    Y_shape = (1,1,2,3,4)
    m = make_model(eq, A_shape, B_shape, Y_shape)
    m2 = lower_einsum(m)
    # should run on CPU without reshape errors
    out = run_inference(m2, A_shape, B_shape)
    assert out[0].shape == Y_shape


def test_lower_einsum_pattern2():
    # hb w i j pattern: h=1,b=1,w=2,i=3,j=2,c=4
    eq = 'hbwij,hbwjc->hbwic'
    A_shape = (1,1,2,3,2)
    B_shape = (1,1,2,2,4)
    Y_shape = (1,1,2,3,4)
    m = make_model(eq, A_shape, B_shape, Y_shape)
    m2 = lower_einsum(m)
    out = run_inference(m2, A_shape, B_shape)
    assert out[0].shape == Y_shape
