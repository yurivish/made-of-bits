gen`a = (b + c) * d`;

// maybe {} is a 'hole' in the AST and we can basically use
// this as a javascript template, which is parsed into an AST
// and then the values are interpolated. Each hole can be filled by
// any JS expression, regardless of whether it makes sense in the context.
// Though it will fail upon interpolation if the result is incoherent, eg.
// 'obj.function foo() { }'
const tpl = field => gen`a.${field} = (b.${field} + c.${field}) * d.${field}`;

fields.map(tpl).map(toJS).join('\n');

// not quite sure where this is heading yet.