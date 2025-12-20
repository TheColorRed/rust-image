"""Compare outputs between CPU and DML providers for a given ONNX model and input size.
Usage: .venv/Scripts/python deployment/compare_cpu_dml.py --model <path> --size 64
"""
import argparse
import numpy as np
import onnxruntime as ort

parser = argparse.ArgumentParser()
parser.add_argument('--model', required=True)
parser.add_argument('--size', type=int, default=64)
args = parser.parse_args()

print('Providers available:', ort.get_available_providers())

# Create sessions explicitly with providers lists
sess_cpu = ort.InferenceSession(args.model, providers=['CPUExecutionProvider'])
sess_dml = None
if 'DmlExecutionProvider' in ort.get_available_providers():
    sess_dml = ort.InferenceSession(args.model, providers=['DmlExecutionProvider', 'CPUExecutionProvider'])
else:
    print('DML provider not available; skipping DML comparison')

inp = sess_cpu.get_inputs()[0]
raw_shape = inp.shape
shape = []
for i, s in enumerate(raw_shape):
    if isinstance(s, int):
        shape.append(s)
        continue
    if i == 0:
        shape.append(1)
    elif i == 1:
        shape.append(3)
    else:
        shape.append(args.size if args.size > 0 else 256)
while len(shape) < 4:
    shape.append(args.size)

print('Input shape:', shape)
x = np.random.randn(*shape).astype('float32')

out_cpu = sess_cpu.run(None, {inp.name: x})
print('CPU outputs:', len(out_cpu))

if sess_dml is None:
    print('DML session unavailable, exiting')
    raise SystemExit(0)

out_dml = sess_dml.run(None, {inp.name: x})
print('DML outputs:', len(out_dml))

# Compare each output
for i, (a, b) in enumerate(zip(out_cpu, out_dml)):
    assert a.shape == b.shape, f"Output {i} shape mismatch: {a.shape} vs {b.shape}"
    diff = a - b
    max_abs = float(np.max(np.abs(diff)))
    max_rel = float(np.max(np.abs(diff) / (np.abs(a) + 1e-8)))
    print(f'Output {i}: shape={a.shape}, max_abs={max_abs:.6e}, max_rel={max_rel:.6e}')

print('Comparison done')
