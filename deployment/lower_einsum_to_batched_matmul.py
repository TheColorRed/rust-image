"""Lower Einsum patterns 'hbwpc,hbwqc->hbwpq' to DML-friendly sequence:
Transpose (if needed) + Reshape to [batch, m, k] and [batch, n, k] + Transpose second to [batch, k, n] + MatMul + Reshape back.

Usage: python lower_einsum_to_batched_matmul.py <in.onnx> <out.onnx>

This is conservative and only transforms Einsum nodes with equation attribute exactly
"hbwpc,hbwqc->hbwpq".
"""
import sys
import onnx
from onnx import helper, TensorProto, numpy_helper


def make_const(name, np_array, dtype=TensorProto.INT64):
    t = numpy_helper.from_array(np_array.astype('int64'))
    t.name = name + '_const'
    node = helper.make_node('Constant', inputs=[], outputs=[name], value=t, name=name + '_node')
    return node


def lower_einsum(model: onnx.ModelProto) -> onnx.ModelProto:
    graph = model.graph
    nodes = list(graph.node)
    new_nodes = []
    idx = 0
    for node in nodes:
        if node.op_type == 'Einsum':
            eq = None
            for a in node.attribute:
                if a.name == 'equation':
                    eq = a.s.decode('utf-8') if isinstance(a.s, bytes) else a.s
            if eq == 'hbwpc,hbwqc->hbwpq' or eq == 'hbwij,hbwjc->hbwic':
                # perform lowering for both patterns (they are the same batched-matmul over the last dim)
                a_in = node.input[0]
                b_in = node.input[1]
                out = node.output[0]
                # make prefix unique to avoid duplicate node/initializer names when lowering multiple times
                prefix = f'lower_{idx}_{abs(id(node)) % 1000000}_'

                # shapeA = Shape(a)
                shapeA = prefix + 'shapeA'
                new_nodes.append(helper.make_node('Shape', [a_in], [shapeA], name=prefix+'shapeA'))

                # shapeB = Shape(b)
                shapeB = prefix + 'shapeB'
                new_nodes.append(helper.make_node('Shape', [b_in], [shapeB], name=prefix+'shapeB'))

                # batch_shape = Slice(shapeA, starts=[0], ends=[3])  -- first 3 dims
                starts = prefix + 'starts'
                ends = prefix + 'ends'
                axes = prefix + 'axes'
                new_nodes.append(helper.make_node('Constant', [], [starts], value=helper.make_tensor(name=starts+'_val', data_type=TensorProto.INT64, dims=[1], vals=[0]), name=starts+'_const'))
                new_nodes.append(helper.make_node('Constant', [], [ends], value=helper.make_tensor(name=ends+'_val', data_type=TensorProto.INT64, dims=[1], vals=[3]), name=ends+'_const'))
                new_nodes.append(helper.make_node('Constant', [], [axes], value=helper.make_tensor(name=axes+'_val', data_type=TensorProto.INT64, dims=[1], vals=[0]), name=axes+'_const'))
                batch_shape = prefix + 'batch_shape'
                new_nodes.append(helper.make_node('Slice', [shapeA, starts, ends, axes], [batch_shape], name=prefix+'batch_slice'))

                # batch_prod = ReduceProd(batch_shape)
                batch_prod = prefix + 'batch_prod'
                new_nodes.append(helper.make_node('ReduceProd', [batch_shape], [batch_prod], name=prefix+'batch_prod', keepdims=0))

                # decide indices for p,q,c depending on equation
                # for 'hbwpc,hbwqc->hbwpq' : p=3, q=3, c=4
                # for 'hbwij,hbwjc->hbwic' : p=3, q=4, c=4
                if eq == 'hbwpc,hbwqc->hbwpq':
                    p_val, q_val, c_val = 3, 3, 4
                else:
                    p_val, q_val, c_val = 3, 4, 4

                idx_p = prefix + 'p_idx'
                idx_q = prefix + 'q_idx'
                idx_c = prefix + 'c_idx'
                # constants for indices
                new_nodes.append(helper.make_node('Constant', [], [idx_p], value=helper.make_tensor(name=idx_p+'_val', data_type=TensorProto.INT64, dims=[1], vals=[p_val]), name=idx_p+'_const'))
                new_nodes.append(helper.make_node('Constant', [], [idx_q], value=helper.make_tensor(name=idx_q+'_val', data_type=TensorProto.INT64, dims=[1], vals=[q_val]), name=idx_q+'_const'))
                new_nodes.append(helper.make_node('Constant', [], [idx_c], value=helper.make_tensor(name=idx_c+'_val', data_type=TensorProto.INT64, dims=[1], vals=[c_val]), name=idx_c+'_const'))

                p = prefix + 'p'
                q = prefix + 'q'
                c = prefix + 'c'
                new_nodes.append(helper.make_node('Gather', [shapeA, idx_p], [p], name=prefix+'gather_p', axis=0))
                # q can come from either shapeA or shapeB depending on pattern; choose the correct source per equation
                if eq == 'hbwpc,hbwqc->hbwpq':
                    # q is in shapeB for this equation
                    new_nodes.append(helper.make_node('Gather', [shapeB, idx_q], [q], name=prefix+'gather_q', axis=0))
                else:
                    # q is in shapeA for the 'hbwij,hbwjc->hbwic' equation
                    new_nodes.append(helper.make_node('Gather', [shapeA, idx_q], [q], name=prefix+'gather_q', axis=0))
                new_nodes.append(helper.make_node('Gather', [shapeB, idx_c], [c], name=prefix+'gather_c', axis=0))

                # Squeeze gathered 1-D tensors to scalars (rank-0), then Unsqueeze to 1-D length-1 for concat
                squeeze_axis_const = prefix + 'squeeze_axes'
                new_nodes.append(helper.make_node('Constant', [], [squeeze_axis_const], value=helper.make_tensor(name=squeeze_axis_const+'_val', data_type=TensorProto.INT64, dims=[1], vals=[0]), name=squeeze_axis_const+'_const'))
                p_s = prefix + 'p_s'
                q_s = prefix + 'q_s'
                c_s = prefix + 'c_s'
                new_nodes.append(helper.make_node('Squeeze', [p, squeeze_axis_const], [p_s], name=prefix+'squeeze_p'))
                new_nodes.append(helper.make_node('Squeeze', [q, squeeze_axis_const], [q_s], name=prefix+'squeeze_q'))
                new_nodes.append(helper.make_node('Squeeze', [c, squeeze_axis_const], [c_s], name=prefix+'squeeze_c'))

                us_axes = prefix + 'us_axes'
                new_nodes.append(helper.make_node('Constant', [], [us_axes], value=helper.make_tensor(name=us_axes+'_val', data_type=TensorProto.INT64, dims=[1], vals=[0]), name=us_axes+'_const'))
                p1 = prefix + 'p1'
                q1 = prefix + 'q1'
                c1 = prefix + 'c1'
                bp1 = prefix + 'bp1'
                new_nodes.append(helper.make_node('Unsqueeze', [p_s, us_axes], [p1], name=prefix+'unsq_p'))
                new_nodes.append(helper.make_node('Unsqueeze', [q_s, us_axes], [q1], name=prefix+'unsq_q'))
                new_nodes.append(helper.make_node('Unsqueeze', [c_s, us_axes], [c1], name=prefix+'unsq_c'))
                new_nodes.append(helper.make_node('Unsqueeze', [batch_prod, us_axes], [bp1], name=prefix+'unsq_batchprod'))

                # For 'hbwpc,hbwqc->hbwpq': A->[batch,p,c], B->[batch,q,c], transpose B->[batch,c,q], MatMul(a,b_t)->[batch,p,q]
                # For 'hbwij,hbwjc->hbwic': A->[batch,p,q], B->[batch,q,c], MatMul(a,b)->[batch,p,c]
                concat_axes = prefix + 'concat_axis'
                new_nodes.append(helper.make_node('Constant', [], [concat_axes], value=helper.make_tensor(name=concat_axes+'_val', data_type=TensorProto.INT64, dims=[1], vals=[0]), name=concat_axes+'_const'))
                newshapeA = prefix + 'shapeA_3d'
                newshapeB = prefix + 'shapeB_3d'
                a_r = prefix + 'a_3d'
                b_r = prefix + 'b_3d'
                mm = prefix + 'matmul'
                newshape_out = prefix + 'shape_out'

                if eq == 'hbwpc,hbwqc->hbwpq':
                    # A -> [batch, p, c]
                    new_nodes.append(helper.make_node('Concat', [bp1, p1, c1], [newshapeA], name=prefix+'concat_shapeA', axis=0))
                    # B -> [batch, q, c]
                    new_nodes.append(helper.make_node('Concat', [bp1, q1, c1], [newshapeB], name=prefix+'concat_shapeB', axis=0))
                    new_nodes.append(helper.make_node('Reshape', [a_in, newshapeA], [a_r], name=prefix+'reshape_a'))
                    new_nodes.append(helper.make_node('Reshape', [b_in, newshapeB], [b_r], name=prefix+'reshape_b'))
                    # transpose b to [batch, c, q]
                    b_t = prefix + 'b_trans'
                    new_nodes.append(helper.make_node('Transpose', [b_r], [b_t], name=prefix+'transpose_b', perm=[0,2,1]))
                    # matmul
                    new_nodes.append(helper.make_node('MatMul', [a_r, b_t], [mm], name=prefix+'matmul'))
                    # result shape: concat(batch_shape, p, q)
                    new_nodes.append(helper.make_node('Concat', [batch_shape, p1, q1], [newshape_out], name=prefix+'concat_out', axis=0))
                else:
                    # eq == 'hbwij,hbwjc->hbwic'
                    # A -> [batch, p, q]
                    new_nodes.append(helper.make_node('Concat', [bp1, p1, q1], [newshapeA], name=prefix+'concat_shapeA', axis=0))
                    # B -> [batch, q, c]
                    new_nodes.append(helper.make_node('Concat', [bp1, q1, c1], [newshapeB], name=prefix+'concat_shapeB', axis=0))
                    new_nodes.append(helper.make_node('Reshape', [a_in, newshapeA], [a_r], name=prefix+'reshape_a'))
                    new_nodes.append(helper.make_node('Reshape', [b_in, newshapeB], [b_r], name=prefix+'reshape_b'))
                    # matmul
                    new_nodes.append(helper.make_node('MatMul', [a_r, b_r], [mm], name=prefix+'matmul'))
                    # result shape: concat(batch_shape, p, c)
                    new_nodes.append(helper.make_node('Concat', [batch_shape, p1, c1], [newshape_out], name=prefix+'concat_out', axis=0))

                # reshape back
                new_nodes.append(helper.make_node('Reshape', [mm, newshape_out], [out], name=prefix+'reshape_out'))

                idx += 1
                continue
        # default: keep node
        new_nodes.append(node)

    # Replace graph nodes
    graph.ClearField('node')
    graph.node.extend(new_nodes)
    return model


if __name__ == '__main__':
    if len(sys.argv) < 3:
        print('Usage: lower_einsum_to_batched_matmul.py in.onnx out.onnx')
        sys.exit(1)
    inp = sys.argv[1]
    outp = sys.argv[2]
    m = onnx.load(inp)
    m2 = lower_einsum(m)
    # optional: infer shapes, skipped for speed
    onnx.save(m2, outp)
    print('Wrote', outp)
