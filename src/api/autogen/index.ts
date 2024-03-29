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
import { fileURLToPath } from 'url';

process.chdir(fileURLToPath(new URL('./', import.meta.url)));

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

function set<K, V>(map: Map<K, V>, key: K, value: V) {
  if (map.has(key)) {
    throw new Error(`Duplicate key: ${key}`);
  }
  map.set(key, value);
}

function assertEmpty(obj: object, msg: string) {
  assert.deepEqual(obj, {}, msg);
}

function toPropName(name: string) {
  // fixup acronyms so that they're not all-caps
  name = name.replaceAll(
    /([A-Z])([A-Z]*)([A-Z][a-z]+)/g,
    (_, a, b, c) => `${a}${b.toLowerCase()}${c}`
  );
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

let types = new Map<
  string,
  {
    features: Set<string>;
    type: RegisteredType;
  }
>();
let typeBySchema = new WeakMap<OpenAPIV3.SchemaObject, RustType>();

function registerType<T extends RegisteredType>(
  devicePath: string,
  schema: OpenAPIV3.SchemaObject,
  createType: (schema: OpenAPIV3.SchemaObject) => T | RustType
): RustType {
  let rustyType = getOrSet(typeBySchema, schema, schema => {
    let type = createType(schema);
    if (type instanceof RustType) {
      return type;
    } else {
      set(types, type.name, { type, features: new Set<string>() });
      return rusty(type.name);
    }
  });
  if (devicePath !== '{device_type}') {
    // This needs to be done even on cached types.
    addFeature(rustyType, devicePath);
  }
  return rustyType;
}

// Recursively add given feature flag to the type.
function addFeature(
  rustyType: RustType,
  feature: string,
  visited = new Set<string>()
) {
  let typeName = rustyType.toString();
  if (visited.has(typeName)) {
    return;
  }
  visited.add(typeName);
  let registeredType = types.get(typeName);
  if (!registeredType) {
    return;
  }
  registeredType.features.add(feature);
  if (
    registeredType.type.kind === 'Enum' ||
    registeredType.type.kind === 'Date'
  ) {
    return;
  }
  for (let { type } of registeredType.type.properties.values()) {
    addFeature(type, feature, visited);
  }
}

class RustType {
  constructor(private rusty: string, public readonly convertVia?: string) {}

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

function rusty(rusty: string, convertVia?: string) {
  return new RustType(rusty, convertVia);
}

interface RegisteredTypeBase {
  name: string;
  doc: string | undefined;
}

type RegisteredType = ObjectType | EnumType | DateType;

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

interface DateType extends RegisteredTypeBase {
  kind: 'Date';
  format: string;
}

interface DeviceMethod {
  name: string;
  mutable: boolean;
  path: string;
  doc: string | undefined;
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
  devicePath: string,
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
        devicePath,
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
  devicePath: string,
  name: string,
  schema: OpenAPIV3.SchemaObject | OpenAPIV3.ReferenceObject = err(
    'Missing schema'
  )
): RustType {
  return withContext(name, () => {
    ({ name = name, target: schema } = nameAndTarget(schema));
    if (schema.enum) {
      return registerType(devicePath, schema, schema => {
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
        return rusty(
          `Vec<${handleType(devicePath, `${name}Item`, schema.items)}>`
        );
      case 'number':
        return rusty('f64');
      case 'string':
        if (schema.format === 'date') {
          let formatter = registerType(devicePath, schema, schema => ({
            name,
            doc: getDoc(schema),
            kind: 'Date',
            format: (schema as any)['date-format']
          }));
          return rusty('std::time::SystemTime', `${formatter}`);
        }
        return rusty('String');
      case 'boolean':
        return rusty('bool');
      case 'object': {
        return registerType(devicePath, schema, schema => ({
          kind: 'Object',
          name,
          doc: getDoc(schema),
          properties: handleObjectProps(devicePath, name, schema)
        }));
      }
    }
    throw new Error(`Unknown type ${schema.type}`);
  });
}

function handleOptType(
  devicePath: string,
  name: string,
  schema: OpenAPIV3.SchemaObject | OpenAPIV3.ReferenceObject | undefined,
  required: boolean
): RustType {
  let type = handleType(devicePath, name, schema);
  return required ? type : rusty(`Option<${type}>`);
}

function handleContent(
  devicePath: string,
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
    if (name.endsWith(baseKind)) {
      name = name.slice(0, -baseKind.length);
    }
    return registerType(devicePath, schema, schema => {
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
        let valueType = handleType(devicePath, name, properties.Value);
        return rusty(
          valueType.toString(),
          valueType.convertVia ?? 'ValueResponse'
        );
      }

      let convertedProps = handleObjectProps(devicePath, name, {
        properties,
        required
      });

      if (baseKind === 'Request') {
        for (let prop of convertedProps.values()) {
          if (prop.type.toString() === 'bool') {
            // Boolean parameters need to be deserialized in special case-insensitive way.
            prop.type = rusty('bool', 'BoolParam');
          }
        }
      }

      return {
        kind: baseKind,
        name,
        doc,
        properties: convertedProps
      };
    });
  });
}

function handleResponse(
  devicePath: string,
  prefixName: string,
  {
    responses: { 200: success, 400: error400, 500: error500, ...otherResponses }
  }: OpenAPIV3.OperationObject
) {
  assertEmpty(otherResponses, 'Unexpected response status codes');
  return handleContent(
    devicePath,
    prefixName,
    'Response',
    'application/json',
    success
  );
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

      for (let param of params.map(resolveMaybeRef)) {
        assert.equal(param?.in, 'query', 'Parameter is not a query parameter');
        let name = toPropName(param.name);
        set(resolvedArgs, name, {
          name,
          originalName: param.name,
          doc: getDoc(param),
          type: handleOptType(
            devicePath,
            `${device.name}${canonicalMethodName}Request${param.name}`,
            param.schema,
            param.required ?? false
          )
        });
      }

      set(device.methods, canonicalMethodName, {
        name: toPropName(canonicalMethodName),
        mutable: false,
        path: methodPath,
        doc: getDoc(get),
        resolvedArgs,
        returnType: handleResponse(
          devicePath,
          `${device.name}${canonicalMethodName}`,
          get
        )
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
        devicePath,
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
          resolvedType.type.kind,
          'Request' as const,
          'Registered type is not a request'
        );
        resolvedArgs = resolvedType.type.properties;
      } else {
        resolvedArgs = new Map();
      }

      set(device.methods, canonicalMethodName, {
        name: toPropName(canonicalMethodName),
        mutable: true,
        path: methodPath,
        doc: getDoc(put),
        resolvedArgs,
        returnType: handleResponse(
          devicePath,
          `${device.name}${canonicalMethodName}`,
          put
        )
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
  return doc
    .split(/\r?\n/)
    .map(line => `/// ${line}`)
    .join('\n');
}

let rendered = `
// DO NOT EDIT. This file is auto-generated by running 'pnpm generate' in the 'src/api/autogen' folder.

/*!
${api.info.title} ${api.info.version}

${api.info.description}
*/

#![allow(
  rustdoc::broken_intra_doc_links,
  clippy::doc_markdown,
  clippy::as_conversions, // triggers on derive-generated code https://github.com/rust-lang/rust-clippy/issues/9657
)]

mod bool_param;
mod devices_impl;
mod server_info;

use bool_param::BoolParam;
use crate::{ASCOMError, ASCOMResult};
use crate::macros::{rpc_mod, rpc_trait};
use crate::response::ValueResponse;
use macro_rules_attribute::apply;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub use server_info::*;

${stringifyIter(types, ({ features, type }) => {
  let cfgs = Array.from(features, feature => `feature = "${feature}"`).join(
    ', '
  );
  let cfg: string;
  switch (features.size) {
    case 0:
      cfg = '';
      break;

    default:
      cfgs = `any(${cfgs})`;
    // fallthrough

    case 1:
      cfg = `#[cfg(${cfgs})]`;
  }

  if (type.name === 'ImageArray') {
    // Override with a better implementation.
    return `
      ${cfg}
      mod image_array;

      ${cfg}
      pub use image_array::*;
    `;
  }

  if (type.name === 'ImageArrayType') {
    return '';
  }

  switch (type.kind) {
    case 'Request':
      return '';
    case 'Object':
    case 'Response': {
      return `
        ${stringifyDoc(type.doc)}
        ${cfg}
        #[allow(missing_copy_implementations)]
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        pub struct ${type.name} {
          ${stringifyIter(
            type.properties,
            prop => `
              ${stringifyDoc(prop.doc)}
              ${
                toPascalCase(prop.name) === prop.originalName &&
                toPropName(prop.originalName) === prop.name
                  ? ''
                  : `#[serde(rename = "${prop.originalName}")]`
              }
              pub ${prop.name}: ${prop.type},
            `
          )}
        }

      `;
    }
    case 'Enum': {
      return `
        ${stringifyDoc(type.doc)}
        ${cfg}
        #[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr, TryFromPrimitive, IntoPrimitive)]
        #[repr(${type.baseType})]
        #[allow(clippy::default_numeric_fallback)] // false positive https://github.com/rust-lang/rust-clippy/issues/9656
        #[allow(missing_docs)] // some enum variants might not have docs and that's okay
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
    case 'Date': {
      return `
        ${stringifyDoc(type.doc)}
        ${cfg}
        #[derive(Debug, Serialize, Deserialize)]
        pub(crate) struct ${type.name} {
          #[serde(rename = "Value", with = "${type.name}")]
          pub(crate) value: time::OffsetDateTime,
        }

        ${cfg}
        impl From<std::time::SystemTime> for ${type.name} {
          fn from(value: std::time::SystemTime) -> Self {
            Self { value: value.into() }
          }
        }

        ${cfg}
        impl From<${type.name}> for std::time::SystemTime {
          fn from(wrapper: ${type.name}) -> Self {
            wrapper.value.into()
          }
        }

        ${cfg}
        impl ${type.name} {
          const FORMAT: &'static [time::format_description::FormatItem<'static>] = time::macros::format_description!("${
            type.format
          }");

          fn serialize<S: serde::Serializer>(value: &time::OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error> {
            value
            .to_offset(time::UtcOffset::UTC)
            .format(Self::FORMAT)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
          }

          fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<time::OffsetDateTime, D::Error> {
            struct Visitor;

            impl serde::de::Visitor<'_> for Visitor {
              type Value = time::OffsetDateTime;

              fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a date string")
              }

              fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                match time::PrimitiveDateTime::parse(value, ${
                  type.name
                }::FORMAT) {
                  Ok(time) => Ok(time.assume_utc()),
                  Err(err) => Err(serde::de::Error::custom(err)),
                }
              }
            }

            deserializer.deserialize_str(Visitor)
          }
        }
      `;
    }
  }
})}

${stringifyIter(
  devices,
  device => `
    ${stringifyDoc(device.doc)}
    ${
      device.path === '{device_type}'
        ? ''
        : `#[cfg(feature = "${device.path}")]`
    } #[apply(rpc_trait)]
    pub trait ${device.name}: ${
    device.path === '{device_type}'
      ? 'std::fmt::Debug + Send + Sync'
      : 'Device + Send + Sync'
  } {
      ${
        device.path === '{device_type}'
          ? `
        const EXTRA_METHODS: () = {
          /// Static device name for the configured list.
          fn static_name(&self) -> &str {
            &self.name
          }

          /// Unique ID of this device.
          fn unique_id(&self) -> &str {
            &self.unique_id
          }
        };
      `
          : ''
      }
      ${stringifyIter(
        device.methods,
        method => `
          ${stringifyDoc(method.doc)}
          #[http("${method.path}", method = ${method.mutable ? 'Put' : 'Get'}${
          method.returnType.convertVia
            ? `, via = ${method.returnType.convertVia}`
            : ''
        })]
          async fn ${method.name}(
            &self,
            ${stringifyIter(
              method.resolvedArgs,
              arg =>
                `
                  #[http("${arg.originalName}"${
                  arg.type.convertVia ? `, via = ${arg.type.convertVia}` : ''
                })]
                  ${arg.name}: ${arg.type},
                `
            )}
          ) -> ASCOMResult${method.returnType.ifNotVoid(type => `<${type}>`)} {
            ${
              method.name.startsWith('can_')
              ? 'Ok(false)'
              : device.path === '{device_type}' && method.name === 'name'
              ? 'Ok(self.static_name().to_owned())'
              : device.path === '{device_type}' && method.name === 'interface_version'
              ? 'Ok(3_i32)'
              : device.path === '{device_type}' && method.name === 'supported_actions'
              ? 'Ok(vec![])'
              : 'Err(ASCOMError::NOT_IMPLEMENTED)'
            }
          }
        `
      )}
    }
  `
)}

rpc_mod! {${stringifyIter(devices, device =>
  device.path === '{device_type}'
    ? ''
    : `
    ${device.name} = "${device.path}",`
)}
}
`;

try {
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
} catch (err) {
  console.warn(err);
}

await writeFile('../mod.rs', rendered);
