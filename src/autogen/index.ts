import openapi from '@readme/openapi-parser';
import { writeFile } from 'fs/promises';
import { spawnSync } from 'child_process';
import {
  toSnakeCase,
  toPascalCase as toTypeName,
  toPascalCase
} from 'js-convert-case';
import { OpenAPIV3 } from 'openapi-types';
import * as assert from 'assert/strict';
import { getCanonicalNames } from './xml-names.js';
import { rustKeywords } from './rust-keywords.js';
import { isDeepStrictEqual } from 'util';

let api = (await openapi.parse(
  './AlpacaDeviceAPI_v1.yaml'
)) as OpenAPIV3.Document;
let _refs = await openapi.resolve(api);
let canonicalNames = await getCanonicalNames('{device_type}');

function err(msg: string): never {
  throw new Error(msg);
}

function getOrSet<K, V>(
  map: Map<K, V> | (K extends object ? WeakMap<K, V> : never),
  key: K,
  createValue: (key: K) => V
): V {
  let value = map.get(key);
  if (value === undefined) {
    map.set(key, (value = createValue(key)));
  }
  return value;
}

function set<K, V, K2 extends K, V2 extends V>(
  map: Map<K, V>,
  key: K2,
  value: V2
) {
  if (map.has(key)) {
    throw new Error(`Duplicate key: ${key}`);
  }
  map.set(key, value);
}

function assertEmpty(obj: object, msg: string) {
  assert.deepEqual(obj, {}, msg);
}

function toPropName(name: string) {
  name = toSnakeCase(name);
  if (rustKeywords.has(name)) name += '_';
  return name;
}

function isRef(maybeRef: any): maybeRef is OpenAPIV3.ReferenceObject {
  return maybeRef != null && '$ref' in maybeRef;
}

function getRef(ref: OpenAPIV3.ReferenceObject): unknown {
  return _refs.get(ref.$ref);
}

function resolveMaybeRef<T>(maybeRef: T | OpenAPIV3.ReferenceObject): T {
  return isRef(maybeRef) ? (getRef(maybeRef) as T) : maybeRef;
}

function nameAndTarget<T>(ref: T | OpenAPIV3.ReferenceObject) {
  if (isRef(ref)) {
    return {
      name: toTypeName(ref.$ref.match(/([^/]+)$/)![1]),
      target: getRef(ref) as T
    };
  } else {
    return { target: ref };
  }
}

function getDoc({
  summary,
  description
}: {
  summary?: string;
  description?: string;
}): string | undefined {
  return description ?? summary;
  // Many descriptions duplicate summary too. Uncomment this below if they're ever improved.
  // return summary && summary + (description ? `\n\n${description}` : '');
}

let types = new Map<string, RegisteredType>();
let typeBySchema = new WeakMap<OpenAPIV3.SchemaObject, RustType>();

function registerType<T extends RegisteredType>(
  schema: OpenAPIV3.SchemaObject,
  createType: (schema: OpenAPIV3.SchemaObject) => T | RustType
): RustType {
  return getOrSet(typeBySchema, schema, schema => {
    let type = createType(schema);
    if (type instanceof RustType) {
      return type;
    } else {
      set(types, type.name, type);
      return rusty(type.name);
    }
  });
}

class RustType {
  constructor(private rusty: string) {}

  isVoid() {
    return this.rusty === '()';
  }

  ifNotVoid(cb: (type: string) => string) {
    return this.isVoid() ? '' : cb(this.rusty);
  }

  toString() {
    return this.rusty;
  }
}

function rusty(rusty: string) {
  return new RustType(rusty);
}

interface RegisteredTypeBase {
  name: string;
  doc: string | undefined;
}

type RegisteredType = ObjectType | EnumType;

interface Property {
  name: string;
  originalName: string;
  type: RustType;
  doc: string | undefined;
}

interface ObjectType extends RegisteredTypeBase {
  kind: 'Object' | 'Request' | 'Response';
  properties: Map<string, Property>;
}

interface EnumVariant {
  doc: string | undefined;
  name: string;
  value: number;
}

interface EnumType extends RegisteredTypeBase {
  kind: 'Enum';
  baseType: RustType;
  variants: Map<string, EnumVariant>;
}

interface DeviceMethod {
  name: string;
  mutable: boolean;
  path: string;
  doc: string | undefined;
  argsType: RustType;
  resolvedArgs: ObjectType['properties'];
  returnType: RustType;
}

interface Device {
  name: string;
  path: string;
  doc: string | undefined;
  methods: Map<string, DeviceMethod>;
}

let devices: Map<string, Device> = new Map();

function withContext<T>(context: string, fn: () => T) {
  try {
    return fn();
  } catch (e) {
    (e as Error).message = `in ${context}:\n${(e as Error).message}`;
    throw e;
  }
}

function handleIntFormat(format: string | undefined): RustType {
  switch (format) {
    case 'uint32':
      return rusty('u32');
    case 'int32':
      return rusty('i32');
    default:
      throw new Error(`Unknown integer format ${format}`);
  }
}

function assertString(value: any): asserts value is string {
  assert.equal(
    typeof value,
    'string',
    `${JSON.stringify(value)} is not a string`
  );
}

function handleObjectProps(
  objName: string,
  {
    properties = err('Missing properties'),
    required = []
  }: Pick<OpenAPIV3.SchemaObject, 'properties' | 'required'>
) {
  let objProperties: ObjectType['properties'] = new Map();
  for (let [propName, propSchema] of Object.entries(properties)) {
    set(objProperties, propName, {
      name: toPropName(propName),
      originalName: propName,
      type: handleOptType(
        `${objName}${propName}`,
        propSchema,
        required.includes(propName)
      ),
      doc: getDoc(resolveMaybeRef(propSchema))
    });
  }
  return objProperties;
}

function handleType(
  name: string,
  schema: OpenAPIV3.SchemaObject | OpenAPIV3.ReferenceObject = err(
    'Missing schema'
  )
): RustType {
  return withContext(name, () => {
    ({ name = name, target: schema } = nameAndTarget(schema));
    if (schema.enum) {
      return registerType(schema, schema => {
        assert.equal(schema.type, 'integer');
        let enumType: EnumType = {
          kind: 'Enum',
          name,
          doc: getDoc(schema),
          baseType: handleIntFormat(schema.format),
          variants: new Map()
        };
        let {
          'x-enum-varnames': names = err('Missing x-enum-varnames'),
          'x-enum-descriptions': descriptions = []
        } = schema as any;
        assert.ok(Array.isArray(names));
        assert.ok(Array.isArray(descriptions));
        for (let [i, value] of schema.enum!.entries()) {
          let name = names[i];
          assertString(name);
          let doc = descriptions[i];
          if (doc === null) {
            doc = undefined;
          }
          if (doc !== undefined) {
            assertString(doc);
          }
          set(enumType.variants, name, {
            name,
            doc,
            value
          });
        }
        return enumType;
      });
    }
    switch (schema.type) {
      case 'integer':
        return handleIntFormat(schema.format);
      case 'array':
        return rusty(`Vec<${handleType(`${name}Item`, schema.items)}>`);
      case 'number':
        return rusty('f64');
      case 'string':
        return rusty('String');
      case 'boolean':
        return rusty('bool');
      case 'object': {
        return registerType(schema, schema => ({
          kind: 'Object',
          name,
          doc: getDoc(schema),
          properties: handleObjectProps(name, schema)
        }));
      }
    }
    throw new Error(`Unknown type ${schema.type}`);
  });
}

function handleOptType(
  name: string,
  schema: OpenAPIV3.SchemaObject | OpenAPIV3.ReferenceObject | undefined,
  required: boolean
): RustType {
  let type = handleType(name, schema);
  return required ? type : rusty(`Option<${type}>`);
}

function handleContent(
  prefixName: string,
  baseKind: 'Request' | 'Response',
  contentType: string,
  body:
    | OpenAPIV3.RequestBodyObject
    | OpenAPIV3.ResponseObject
    | OpenAPIV3.ReferenceObject = err('Missing content')
): RustType {
  let name = `${prefixName}${baseKind}`;
  return withContext(name, () => {
    ({ name = name, target: body } = nameAndTarget(body));
    let doc = getDoc(body);
    let {
      [contentType]: { schema = err('Missing schema') } = err(
        `Missing ${contentType}`
      ),
      ...otherContentTypes
    } = body.content ?? err('Missing content');
    assertEmpty(otherContentTypes, 'Unexpected types');
    let baseRef = `#/components/schemas/Alpaca${baseKind}`;
    if (isRef(schema) && schema.$ref === baseRef) {
      return rusty('()');
    }
    ({ name = name, target: schema } = nameAndTarget(schema));
    return registerType(schema, schema => {
      doc = getDoc(schema) ?? doc;
      let {
        allOf: [base, extension, ...otherItemsInAllOf] = err('Missing allOf'),
        ...otherPropsInSchema
      } = schema;
      assert.deepEqual(otherItemsInAllOf, [], 'Unexpected items in allOf');
      assertEmpty(
        otherPropsInSchema,
        'Unexpected properties in content schema'
      );
      assert.ok(isRef(base));
      assert.equal(base.$ref, baseRef);
      assert.ok(extension && !isRef(extension));
      let { properties, required, ...otherPropsInExtension } = extension;
      assertEmpty(otherPropsInExtension, 'Unexpected properties in extension');
      // Special-case value responses.
      if (
        baseKind === 'Response' &&
        properties !== undefined &&
        isDeepStrictEqual(Object.keys(properties), ['Value'])
      ) {
        return handleType(name, properties.Value);
      }

      return {
        kind: baseKind,
        name,
        doc,
        properties: handleObjectProps(name, { properties, required })
      };
    });
  });
}

function handleResponse(
  prefixName: string,
  {
    responses: { 200: success, 400: error400, 500: error500, ...otherResponses }
  }: OpenAPIV3.OperationObject
) {
  assertEmpty(otherResponses, 'Unexpected response status codes');
  return handleContent(prefixName, 'Response', 'application/json', success);
}

for (let [path, methods = err('Missing methods')] of Object.entries(
  api.paths
)) {
  // ImageArrayVariant is a semi-deprecated endpoint. Its handling is somewhat
  // complicated, so just skip it until someone requests to implement it.
  if (path === '/camera/{device_number}/imagearrayvariant') {
    continue;
  }

  withContext(`path ${path}`, () => {
    let [, devicePath = err('unreachable'), methodPath = err('unreachable')] =
      path.match(/^\/([^/]*)\/\{device_number\}\/([^/]*)$/) ??
      err('Invalid path');

    let canonicalDevice = canonicalNames.getDevice(devicePath);

    let device = getOrSet<string, Device>(devices, devicePath, () => ({
      name: canonicalDevice.name,
      path: devicePath,
      doc: undefined,
      methods: new Map()
    }));

    let { get, put, ...other } = methods;
    assert.deepEqual(Object.keys(other), [], 'Unexpected methods');

    for (let method of [get, put]) {
      if (!method) continue;
      let [tag, ...otherTags] = method.tags ?? err('Missing tags');
      assert.deepEqual(otherTags, [], 'Unexpected tags');
      if (device.doc !== undefined) {
        assert.equal(device.doc, tag);
      } else {
        device.doc = tag;
      }
    }

    withContext('GET', () => {
      if (!get) return;

      let params = (get.parameters ?? err('Missing parameters')).slice();

      let expectedParams = [
        'device_number',
        'ClientIDQuery',
        'ClientTransactionIDQuery'
      ];
      if (devicePath === '{device_type}') {
        expectedParams.push('device_type');
      }
      for (let expectedParam of expectedParams) {
        let param = params.findIndex(
          param =>
            isRef(param) &&
            param.$ref === `#/components/parameters/${expectedParam}`
        );
        assert.ok(param !== -1, `Missing parameter ${expectedParam}`);
        params.splice(param, 1);
      }

      assert.ok(!get.requestBody);

      let canonicalMethodName = canonicalDevice.getMethod(methodPath);

      let resolvedArgs = new Map<string, Property>();

      let argsType =
        params.length === 0
          ? rusty('()')
          : registerType({}, () => {
              let argsType: ObjectType = {
                kind: 'Request',
                properties: resolvedArgs,
                name: `${device.name}${canonicalMethodName}Request`,
                doc: undefined
              };

              for (let param of params.map(resolveMaybeRef)) {
                assert.equal(
                  param?.in,
                  'query',
                  'Parameter is not a query parameter'
                );
                let name = toPropName(param.name);
                set(resolvedArgs, name, {
                  name,
                  originalName: param.name,
                  doc: getDoc(param),
                  type: handleOptType(
                    `${argsType.name}${param.name}`,
                    param.schema,
                    param.required ?? false
                  )
                });
              }

              return argsType;
            });

      set(device.methods, canonicalMethodName, {
        name: toPropName(canonicalMethodName),
        mutable: false,
        path: methodPath,
        doc: getDoc(get),
        argsType,
        resolvedArgs,
        returnType: handleResponse(`${device.name}${canonicalMethodName}`, get)
      });
    });

    withContext('PUT', () => {
      if (!put) return;

      let params = (put.parameters ?? err('Missing parameters')).slice();

      let expectedParams = ['device_number'];
      if (devicePath === '{device_type}') {
        expectedParams.push('device_type');
      }
      for (let expectedParam of expectedParams) {
        let param = params.findIndex(
          param =>
            isRef(param) &&
            param.$ref === `#/components/parameters/${expectedParam}`
        );
        assert.ok(
          param !== -1,
          `Missing parameter ${expectedParam} in ${JSON.stringify(params)}`
        );
        params.splice(param, 1);
      }
      assert.deepEqual(params, []);

      // If there's a getter, then this is a setter and needs to be prefixed with `Set`.
      let canonicalMethodName =
        (get ? 'Set' : '') + canonicalDevice.getMethod(methodPath);

      let argsType = handleContent(
        `${device.name}${canonicalMethodName}`,
        'Request',
        'application/x-www-form-urlencoded',
        put.requestBody
      );

      let resolvedArgs;

      if (!argsType.isVoid()) {
        let resolvedType = types.get(argsType.toString());
        assert.ok(resolvedType, 'Could not find registered type');
        assert.equal(
          resolvedType.kind,
          'Request' as const,
          'Registered type is not a request'
        );
        resolvedArgs = resolvedType.properties;
      } else {
        resolvedArgs = new Map();
      }

      set(device.methods, canonicalMethodName, {
        name: toPropName(canonicalMethodName),
        mutable: true,
        path: methodPath,
        doc: getDoc(put),
        argsType,
        resolvedArgs,
        returnType: handleResponse(`${device.name}${canonicalMethodName}`, put)
      });
    });
  });
}

function stringifyIter<T>(
  iter: { values(): Iterable<T> },
  stringify: (t: T) => string
) {
  return Array.from(iter.values()).map(stringify).join('');
}

function stringifyDoc(doc: string | undefined = '') {
  doc = doc.trim();
  if (!doc) return '';
  return doc.includes('\n') ? `/**\n${doc}\n*/` : `/// ${doc}`;
}

let rendered = `
// This file is auto-generated. Do not edit it directly.

/*!
${api.info.title} ${api.info.version}

${api.info.description}
*/

#![allow(
  rustdoc::broken_intra_doc_links,
  clippy::doc_markdown,
  clippy::as_conversions, // triggers on derive-generated code https://github.com/rust-lang/rust-clippy/issues/9657
)]

use crate::rpc::rpc;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

${stringifyIter(types, type => {
  if (type.name === 'ImageArrayResponse') {
    // Override with a better implementation.
    return `
      #[path = "image_array_response.rs"]
      mod image_array_response;

      pub use image_array_response::*;
    `;
  }

  switch (type.kind) {
    case 'Object':
    case 'Request':
    case 'Response': {
      let vis = type.kind !== 'Request' ? 'pub' : '';

      return `
        ${stringifyDoc(type.doc)}
        #[allow(missing_copy_implementations)]
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        ${vis} struct ${type.name} {
          ${stringifyIter(
            type.properties,
            prop => `
              ${stringifyDoc(prop.doc)}
              ${
                toPascalCase(prop.name) === prop.originalName &&
                toSnakeCase(prop.originalName) === prop.name
                  ? ''
                  : `#[serde(rename = "${prop.originalName}")]`
              }
              ${vis} ${prop.name}: ${prop.type},
            `
          )}
        }

      `;
    }
    case 'Enum': {
      return `
        ${stringifyDoc(type.doc)}
        #[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
        #[repr(${type.baseType})]
        #[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
        pub enum ${type.name} {
          ${stringifyIter(
            type.variants,
            variant => `
              ${stringifyDoc(variant.doc)}
              ${variant.name} = ${variant.value},
            `
          )}
        }
      `;
    }
  }
})}

rpc! {
  ${stringifyIter(
    devices,
    device => `
      ${stringifyDoc(device.doc)}
      #[http("${device.path}")]
      pub trait ${device.name} {
        ${stringifyIter(
          device.methods,
          method => `
            ${stringifyDoc(method.doc)}
            #[http("${method.path}"${method.argsType.ifNotVoid(
            argsType => `, ${argsType}`
          )})]
            fn ${method.name}(
              &${method.mutable ? 'mut ' : ''}self,
              ${stringifyIter(
                method.resolvedArgs,
                arg => `${arg.name}: ${arg.type},`
              )}
            )${method.returnType.ifNotVoid(type => ` -> ${type}`)};

          `
        )}
      }
    `
  )}
}
`;

// Help rustfmt format contents of the `rpc!` macro.
rendered = rendered.replaceAll('rpc!', 'mod __rpc__');

let rustfmt = spawnSync('rustfmt', ['--edition=2021'], {
  encoding: 'utf-8',
  input: rendered
});
if (rustfmt.error) {
  throw rustfmt.error;
}
if (rustfmt.status !== 0) {
  throw new Error(rustfmt.stderr);
}
rendered = rustfmt.stdout;

// Revert the helper changes.
rendered = rendered.replaceAll('mod __rpc__', 'rpc!');

await writeFile('mod.rs', rendered);
