import openapi from '@readme/openapi-parser';
import { chmod, open, unlink, writeFile } from 'fs/promises';
import { spawnSync } from 'child_process';
import {
  toSnakeCase,
  toPascalCase as toTypeName,
  toPascalCase
} from 'js-convert-case';
import { OpenAPIV3_1, OpenAPIV3 } from 'openapi-types';
import * as assert from 'assert/strict';
import { CanonicalDevice, getCanonicalNames } from './xml-names.js';
import { rustKeywords } from './rust-keywords.js';
import { isDeepStrictEqual } from 'util';
import { fileURLToPath } from 'url';

type ReferenceObject = OpenAPIV3.ReferenceObject;
type SchemaObject = OpenAPIV3_1.SchemaObject;

process.chdir(fileURLToPath(new URL('./', import.meta.url)));

let api = (await openapi.parse(
  './AlpacaDeviceAPI_v1.yaml'
)) as OpenAPIV3_1.Document;
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

class NamedSet<T extends { name: string }> extends Map<string, T> {
  add(value: T) {
    let { name } = value;
    if (this.has(name)) {
      throw new Error(`Item with name ${name} is already in the set`);
    }
    this.set(name, value);
  }

  toString(stringifyItem?: (t: T) => string) {
    return Array.from(this.values(), stringifyItem!).join('');
  }
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

function isRef(maybeRef: any): maybeRef is ReferenceObject {
  return maybeRef != null && '$ref' in maybeRef;
}

function getRef(ref: ReferenceObject): unknown {
  return _refs.get(ref.$ref);
}

function resolveMaybeRef<T>(maybeRef: T | ReferenceObject): T {
  return isRef(maybeRef) ? (getRef(maybeRef) as T) : maybeRef;
}

function nameAndTarget<T>(ref: T | ReferenceObject) {
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

let types = new NamedSet<RegisteredType>();
let typeBySchema = new WeakMap<SchemaObject, RustType>();

function registerType<S extends SchemaObject, T extends RegisteredType>(
  device: Device,
  schema: S,
  createType: (schema: S) => T | RustType
): RustType {
  let rustyType = getOrSet(typeBySchema, schema, schema => {
    let type = createType(schema as S);
    if (type instanceof RustType) {
      return type;
    } else {
      types.add(type);
      return rusty(type.name);
    }
  });
  if (!device.isBaseDevice) {
    // This needs to be done even on cached types.
    types.get(rustyType.toString())?.features.add(device.path);
  }
  return rustyType;
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

abstract class RegisteredTypeBase {
  constructor(
    public readonly name: string,
    public readonly doc: string | undefined
  ) {}

  public readonly features = new Set<string>();

  protected _brand!: never;

  protected stringifyCfg() {
    let cfgs = Array.from(
      this.features,
      feature => `feature = "${feature}"`
    ).join(', ');

    switch (this.features.size) {
      case 0:
        return '';

      case 1:
        return `#[cfg(${cfgs})] `;

      default:
        return `#[cfg(any(${cfgs}))] `;
    }
  }
}

type RegisteredType = ObjectType | EnumType;

class Property {
  public readonly name: string;

  constructor(
    public readonly originalName: string,
    public readonly type: RustType,
    public readonly doc: string | undefined
  ) {
    this.name = toPropName(originalName);
  }

  private _brand!: never;

  toString() {
    return `
      ${stringifyDoc(this.doc)}
      ${
        toPascalCase(this.name) === this.originalName
          ? ''
          : `#[serde(rename = "${this.originalName}")]`
      }
      pub ${this.name}: ${this.type},
    `;
  }
}

class ObjectType extends RegisteredTypeBase {
  public readonly properties = new NamedSet<Property>();

  constructor(
    typeCtx: TypeContext,
    name: string,
    schema: OpenAPIV3_1.NonArraySchemaObject
  ) {
    super(name, getDoc(schema));

    let { properties = err('Missing properties'), required = [] } = schema;
    for (let [propName, propSchema] of Object.entries(properties)) {
      this.properties.add(
        new Property(
          propName,
          typeCtx.handleOptType(
            `${name}${propName}`,
            propSchema,
            required.includes(propName)
          ),
          getDoc(resolveMaybeRef(propSchema))
        )
      );
    }
  }

  toString() {
    let maybeCopy = this.name !== 'DeviceStateItem' ? ', Copy' : '';

    return `
        ${stringifyDoc(this.doc)}
        ${this.stringifyCfg()}#[derive(Debug, Clone${maybeCopy}, Serialize, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        pub struct ${this.name} {
          ${this.properties}
        }
      `;
  }
}

class RequestType extends ObjectType {
  // Requests are inlined into the method signature, so they don't need to be generated as types.
  toString(): string {
    return '';
  }
}

class EnumVariant {
  public readonly name: string;
  public readonly value: number;
  public readonly doc: string | undefined;

  constructor(entry: SchemaObject) {
    assert.ok(!isRef(entry));
    assert.ok(Number.isSafeInteger(entry.const));
    this.name = entry.title ?? err('Missing title');
    this.value = entry.const;
    this.doc = entry.description;
  }

  private _brand!: never;

  toString() {
    return `
      ${stringifyDoc(this.doc)}
      ${this.name} = ${this.value},
    `;
  }
}

class EnumType extends RegisteredTypeBase {
  public readonly variants = new NamedSet<EnumVariant>();
  public readonly baseType: RustType;

  constructor(name: string, schema: SchemaObject) {
    super(name, getDoc(schema));
    this.baseType = handleIntFormat(schema.format);
    assert.ok(Array.isArray(schema.oneOf));
    for (let entry of schema.oneOf) {
      this.variants.add(new EnumVariant(entry));
    }
  }

  toString() {
    return `
      ${stringifyDoc(this.doc)}
      ${this.stringifyCfg()}#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr, TryFromPrimitive, IntoPrimitive)]
      #[repr(${this.baseType})]
      #[allow(missing_docs)] // some enum variants might not have docs and that's okay
      pub enum ${this.name} {
        ${this.variants}
      }
    `;
  }
}

class DeviceMethod {
  public readonly name: string;
  public readonly doc: string | undefined;
  public readonly returnType: RustType;
  public resolvedArgs = new NamedSet<Property>();
  public readonly method: 'GET' | 'PUT';
  private readonly inBaseDevice: boolean;

  constructor(
    device: Device,
    method: 'GET' | 'PUT' | 'PUT(SETTER)',
    public readonly path: string,
    schema: OpenAPIV3_1.OperationObject
  ) {
    let name = toPropName(device.canonical.getMethod(path));
    // If there's a getter, then this is a setter and needs to be prefixed with `set_`.
    if (method === 'PUT(SETTER)') {
      method = 'PUT';
      name = `set_${name}`;
    }
    this.name = name;
    this.method = method;
    this.doc = getDoc(schema);
    this.inBaseDevice = device.isBaseDevice;
    const {
      responses: {
        200: success,
        400: error400,
        500: error500,
        ...otherResponses
      } = err('Missing responses')
    } = schema;
    assertEmpty(otherResponses, 'Unexpected response status codes');
    assert.deepEqual(error400, { $ref: '#/components/responses/400' });
    assert.deepEqual(error500, { $ref: '#/components/responses/500' });
    this.returnType = new TypeContext(method, 'Response', device).handleContent(
      name,
      'application/json',
      success
    );
  }

  private _brand!: never;

  toString() {
    let transformedMethod = (
      {
        GET: 'Get',
        PUT: 'Put'
      } as const
    )[this.method];

    let maybeVia = this.returnType.convertVia
      ? `, via = ${this.returnType.convertVia}`
      : '';

    return `
      ${stringifyDoc(this.doc)}
      #[http("${this.path}", method = ${transformedMethod}${maybeVia})]
      async fn ${this.name}(
        &self,
        ${this.resolvedArgs.toString(
          arg =>
            `
              #[http("${arg.originalName}"${
              arg.type.convertVia ? `, via = ${arg.type.convertVia}` : ''
            })]
              ${arg.name}: ${arg.type},
            `
        )}
      ) -> ASCOMResult${this.returnType.ifNotVoid(type => `<${type}>`)} {
        ${
          this.name.startsWith('can_')
            ? 'Ok(false)'
            : this.inBaseDevice && this.name === 'name'
            ? 'Ok(self.static_name().to_owned())'
            : this.inBaseDevice && this.name === 'interface_version'
            ? 'Ok(3_i32)'
            : this.inBaseDevice && this.name === 'supported_actions'
            ? 'Ok(vec![])'
            : 'Err(ASCOMError::NOT_IMPLEMENTED)'
        }
      }
    `;
  }
}

class Device {
  public readonly canonical: CanonicalDevice;
  public doc: string | undefined = undefined;
  public readonly methods = new NamedSet<DeviceMethod>();
  public readonly isBaseDevice: boolean;

  constructor(public readonly path: string) {
    this.canonical = canonicalNames.getDevice(this.path);
    this.isBaseDevice = this.path === '{device_type}';
  }

  public get name() {
    return this.canonical.name;
  }

  private _brand!: never;

  toString() {
    return `
      ${stringifyDoc(this.doc)}
      ${
        this.isBaseDevice ? '' : `#[cfg(feature = "${this.path}")]`
      } #[apply(rpc_trait)]
      pub trait ${this.name}: ${
      this.isBaseDevice
        ? 'std::fmt::Debug + Send + Sync'
        : 'Device + Send + Sync'
    } {
        ${
          this.isBaseDevice
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
        ${this.methods}
      }
    `;
  }
}

let devices = new NamedSet<Device>();

function withContext<T>(context: string, fn: () => T) {
  Object.defineProperty(fn, 'name', { value: context });
  return fn();
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

class TypeContext {
  constructor(
    private readonly method: 'GET' | 'PUT',
    private readonly baseKind: 'Request' | 'Response',
    private readonly device: Device
  ) {}

  handleType(
    name: string,
    schema: SchemaObject | ReferenceObject = err('Missing schema')
  ): RustType {
    return withContext(name, () => {
      ({ name = name, target: schema } = nameAndTarget(schema));
      switch (schema.type) {
        case 'integer':
          if (schema.oneOf) {
            return registerType(
              this.device,
              schema,
              schema => new EnumType(name, schema)
            );
          }
          return handleIntFormat(schema.format);
        case 'array':
          return rusty(`Vec<${this.handleType(`${name}Item`, schema.items)}>`);
        case 'number':
          return rusty('f64');
        case 'string': {
          let { format } = schema;
          if (format === 'date-time' || format === 'date-time-fits') {
            format = format === 'date-time' ? 'Iso8601' : 'Fits';
            let viaType =
              this.baseKind === 'Request' ? 'TimeParam' : 'TimeResponse';
            return rusty(
              'std::time::SystemTime',
              `time_repr::${viaType}<time_repr::${format}>`
            );
          }
          return rusty('String');
        }
        case 'boolean':
          return rusty(
            'bool',
            this.method === 'GET' && this.baseKind === 'Request'
              ? 'BoolParam'
              : undefined
          );
        case 'object': {
          return registerType(
            this.device,
            schema,
            schema => new ObjectType(this, name, schema)
          );
        }
      }
      if (name === 'DeviceStateItemValue') {
        // This is a variadic type, handle it manually by forwarding to serde_json.
        return rusty('serde_json::Value');
      }
      throw new Error(`Unknown type ${schema.type}`);
    });
  }

  handleOptType(
    name: string,
    schema: SchemaObject | ReferenceObject | undefined,
    required: boolean
  ): RustType {
    let type = this.handleType(name, schema);
    return required ? type : rusty(`Option<${type}>`);
  }

  handleContent(
    canonicalMethodName: string,
    contentType: string,
    body:
      | OpenAPIV3_1.RequestBodyObject
      | OpenAPIV3_1.ResponseObject
      | ReferenceObject = err('Missing content')
  ): RustType {
    let { baseKind } = this;
    let name = `${this.device.name}${canonicalMethodName}${baseKind}`;
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
      return registerType(this.device, schema, schema => {
        if (name === 'ImageArray') {
          return rusty('ImageArray');
        }

        doc = getDoc(schema) ?? doc;
        let {
          allOf: [base, extension, ...otherItemsInAllOf] = err('Missing allOf'),
          ...otherPropsInSchema
        } = schema as any;
        assert.deepEqual(otherItemsInAllOf, [], 'Unexpected items in allOf');
        assertEmpty(
          otherPropsInSchema,
          'Unexpected properties in content schema'
        );
        assert.ok(isRef(base));
        assert.equal(base.$ref, baseRef);
        assert.ok(extension && !isRef(extension));
        let { properties, required, ...otherPropsInExtension } = extension;
        assertEmpty(
          otherPropsInExtension,
          'Unexpected properties in extension'
        );
        // Special-case value responses.
        if (
          baseKind === 'Response' &&
          properties !== undefined &&
          isDeepStrictEqual(Object.keys(properties), ['Value'])
        ) {
          let valueType = this.handleType(name, properties.Value);
          return rusty(
            valueType.toString(),
            valueType.convertVia ?? 'ValueResponse<_>'
          );
        }

        const ctor = baseKind === 'Request' ? RequestType : ObjectType;

        return new ctor(this, name, {
          properties,
          required,
          description: doc
        });
      });
    });
  }
}

function extractParams(
  methodSchema: OpenAPIV3_1.OperationObject,
  device: Device,
  extraExpectedParams: string[]
) {
  let params = (methodSchema.parameters ?? err('Missing parameters')).slice();
  let expectedParams = [...extraExpectedParams, 'device_number'];
  if (device.isBaseDevice) {
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
  return params;
}

for (let [path, methods = err('Missing methods')] of Object.entries(
  api.paths ?? err('Missing paths')
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

    let device = getOrSet(devices, devicePath, () => new Device(devicePath));

    let { get, put, ...other } = methods;
    assertEmpty(other, 'Unexpected methods');

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

      let params = extractParams(get, device, [
        'ClientIDQuery',
        'ClientTransactionIDQuery'
      ]);

      let method = new DeviceMethod(device, 'GET', methodPath, get);

      let paramCtx = new TypeContext('GET', 'Request', device);

      for (let param of params.map(resolveMaybeRef)) {
        assert.ok(!isRef(param));
        assert.equal(param?.in, 'query', 'Parameter is not a query parameter');
        method.resolvedArgs.add(
          new Property(
            param.name,
            paramCtx.handleOptType(
              `${device.name}${method.name}Request${param.name}`,
              param.schema,
              param.required ?? false
            ),
            getDoc(param)
          )
        );
      }

      device.methods.add(method);
    });

    withContext('PUT', () => {
      if (!put) return;

      let params = extractParams(put, device, []);
      assert.deepEqual(params, [], 'Unexpected parameters in PUT method');

      let method = new DeviceMethod(
        device,
        get ? 'PUT(SETTER)' : 'PUT',
        methodPath,
        put
      );

      let argsType = new TypeContext('PUT', 'Request', device).handleContent(
        method.name,
        'application/x-www-form-urlencoded',
        put.requestBody
      );

      if (!argsType.isVoid()) {
        let resolvedType = types.get(argsType.toString());
        assert.ok(resolvedType, 'Could not find registered type');
        assert.ok(
          resolvedType instanceof RequestType,
          'Registered type is not a request'
        );
        method.resolvedArgs = resolvedType.properties;
      }

      device.methods.add(method);
    });
  });
}

function stringifyDoc(doc: string | undefined = '') {
  doc = doc.trim();
  if (!doc) return '';
  return (
    doc
      // Change "`InterfaceV1 Only` ...actual description" to be "actual description\n\n_InterfaceV1 Only_"
      .replace(/^`(.+?)\.?`\s*(.*)$/s, '$2\n\n_$1._')
      // If there is no summary, split out first sentence as summary.
      .replace(/^(.*?(?<!e\.g|i\.e)\.) (?=[A-Z])/, '$1\n\n')
      // Add doc-comment markers to each line.
      .replace(/^/gm, '/// ')
      /// Finish with a period.
      .replace(/(?<!\.)$/, '.')
  );
}

let rendered = `
// DO NOT EDIT. This file is auto-generated by running 'pnpm generate' in the 'src/api/autogen' folder.

/*!
${api.info.title} ${api.info.version}

${api.info.description}
*/

#![allow(clippy::doc_markdown)]

mod bool_param;
mod devices_impl;
mod server_info;
mod time_repr;

use bool_param::BoolParam;
use crate::{ASCOMError, ASCOMResult};
use crate::macros::{rpc_mod, rpc_trait};
use crate::response::ValueResponse;
use macro_rules_attribute::apply;
#[cfg_attr(not(feature = "all-devices"), allow(unused_imports))]
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
#[cfg_attr(not(feature = "all-devices"), allow(unused_imports))]
use serde_repr::{Deserialize_repr, Serialize_repr};

pub(crate) use devices_impl::*;

pub use server_info::*;

#[cfg(feature = "camera")]
mod image_array;

#[cfg(feature = "camera")]
pub use image_array::*;

${types}

${devices}

rpc_mod! {${devices.toString(device =>
  device.isBaseDevice
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

await unlink('../mod.rs');
await writeFile('../mod.rs', rendered, {
  flag: 'wx',
  mode: /* readonly */ 0o444
});
