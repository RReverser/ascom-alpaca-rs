import openapi from '@readme/openapi-parser';
import { unlink, writeFile } from 'fs/promises';
import { spawnSync } from 'child_process';
import { toSnakeCase, toPascalCase } from 'js-convert-case';
import { OpenAPIV3, type OpenAPIV3_1 } from 'openapi-types';
import * as assert from 'assert/strict';
import { CanonicalDevice, canonicalDevices } from './xml-names.ts';
import { rustKeywords } from './rust-keywords.ts';
import { isDeepStrictEqual } from 'util';
import { fileURLToPath } from 'url';

type ReferenceObject = OpenAPIV3.ReferenceObject;
type SchemaObject = OpenAPIV3_1.SchemaObject;

process.chdir(fileURLToPath(new URL('./', import.meta.url)));

let api = (await openapi.parse(
  './AlpacaDeviceAPI_v1.yaml',
)) as OpenAPIV3_1.Document;
let _refs = await openapi.resolve(api);

function err(msg: string): never {
  throw new Error(msg);
}

function getOrSet<K, V>(
  map: Map<K, V> | (K extends object ? WeakMap<K, V> : never),
  key: K,
  createValue: (key: K) => V,
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
    (_, a, b, c) => `${a}${b.toLowerCase()}${c}`,
  );
  name = toSnakeCase(name);
  if (rustKeywords.has(name)) name += '_';
  return name;
}

function nameAndTarget<T>(ref: T) {
  let { $ref } = ref as ReferenceObject;
  return {
    target: ($ref ? _refs.get($ref) : ref) as Exclude<T, ReferenceObject>,
    name: $ref && toPascalCase($ref.match(/([^/]+)$/)![1]),
  };
}

function getDoc({
  summary,
  description,
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
  createType: (schema: S) => T | RustType,
): RustType {
  let rustyType = getOrSet(typeBySchema, schema, (schema) => {
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
  private rusty: string;
  public readonly convertVia?: string;

  constructor(rusty: string, convertVia?: string) {
    this.rusty = rusty;
    this.convertVia = convertVia;
  }

  isVoid() {
    return this.rusty === '()';
  }

  ifNotVoid(cb: (type: string) => string) {
    return this.isVoid() ? '' : cb(this.rusty);
  }

  toString() {
    return this.rusty;
  }

  maybeVia() {
    return this.convertVia ? `, via = ${this.convertVia}` : '';
  }
}

function rusty(rusty: string, convertVia?: string) {
  return new RustType(rusty, convertVia);
}

abstract class RegisteredTypeBase {
  public readonly name: string;
  public readonly doc?: string;

  constructor(name: string, doc?: string) {
    this.name = name;
    this.doc = doc;
  }

  public readonly features = new Set<string>();

  protected _brand!: never;

  protected stringifyCfg() {
    let cfgs = Array.from(
      this.features,
      (feature) => `feature = "${feature}"`,
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
  public readonly originalName: string;
  public readonly type: RustType;
  public readonly doc?: string;
  public readonly name: string;

  constructor(originalName: string, type: RustType, doc?: string) {
    this.originalName = originalName;
    this.type = type;
    this.doc = doc;
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
    schema: OpenAPIV3_1.NonArraySchemaObject,
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
            required.includes(propName),
          ),
          getDoc(nameAndTarget(propSchema).target),
        ),
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
  public readonly doc?: string;

  constructor(entry: SchemaObject) {
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
  public readonly device: Device;
  public readonly path: string;
  public readonly name: string;
  public readonly doc?: string;
  public readonly returnType: RustType;
  public resolvedArgs = new NamedSet<Property>();
  public readonly method: 'Get' | 'Put';

  constructor(
    device: Device,
    method: 'Get' | 'Put' | 'Put(Setter)',
    path: string,
    schema: OpenAPIV3_1.OperationObject,
  ) {
    this.device = device;
    this.path = path;
    let name = toPropName(device.canonical.getMethod(path));
    // If there's a getter, then this is a setter and needs to be prefixed with `set_`.
    if (method === 'Put(Setter)') {
      method = 'Put';
      name = `set_${name}`;
    }
    this.name = name;
    this.method = method;
    this.doc = getDoc(schema)?.replace(/\[`(\w+)`\]\(#\/.*?\)/g, (_, name) => {
      // replace intra-link references with Rust method references
      name = toPropName(device.canonical.getMethod(name));
      return `[\`${name}\`](Self::${name})`;
    });
    const {
      responses: {
        200: success,
        400: error400,
        500: error500,
        ...otherResponses
      } = err('Missing responses'),
    } = schema;
    assertEmpty(otherResponses, 'Unexpected response status codes');
    assert.deepEqual(error400, { $ref: '#/components/responses/400' });
    assert.deepEqual(error500, { $ref: '#/components/responses/500' });
    this.returnType = new TypeContext('Response', device).handleContent(
      name,
      'application/json',
      success,
    );
    device.updateDocFromMethodTags(schema);
  }

  private _brand!: never;

  private getDefaultImpl() {
    switch (this.name) {
      case 'interface_version':
        assert.ok(!this.device.isBaseDevice);
        return `Ok(${this.device.canonical.version}_i32)`;
      case 'name':
        assert.ok(this.device.isBaseDevice);
        return 'Ok(self.static_name().to_owned())';
      case 'supported_actions':
        assert.ok(this.device.isBaseDevice);
        return 'Ok(vec![])';
      default:
        if (this.name.startsWith('can_')) {
          return 'Ok(false)';
        }
        return 'Err(ASCOMError::NOT_IMPLEMENTED)';
    }
  }

  toString() {
    return `
      ${stringifyDoc(this.doc)}
      #[http("${this.path}", method = ${
        this.method
      }${this.returnType.maybeVia()})]
      async fn ${this.name}(
        &self,
        ${this.resolvedArgs.toString(
          (arg) => `
            #[http("${arg.originalName}"${arg.type.maybeVia()})]
            ${arg.name}: ${arg.type},
          `,
        )}
      ) -> ASCOMResult${this.returnType.ifNotVoid((type) => `<${type}>`)} {
        ${this.getDefaultImpl()}
      }
    `;
  }
}

class Device {
  public readonly path: string;
  public readonly canonical: CanonicalDevice;
  public doc?: string;
  public readonly methods = new NamedSet<DeviceMethod>();
  public readonly isBaseDevice: boolean;
  public readonly name: string;

  constructor(path: string) {
    this.path = path;
    this.isBaseDevice = path === '{device_type}';
    this.canonical = canonicalDevices.getDevice(path);
    this.name = this.isBaseDevice ? 'Device' : this.canonical.name;
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
        ${this.methods}
      }
    `;
  }

  updateDocFromMethodTags(method: OpenAPIV3_1.OperationObject) {
    let [tag, ...otherTags] = method.tags ?? err('Missing tags');
    assert.deepEqual(otherTags, [], 'Unexpected tags');
    if (this.doc !== undefined) {
      assert.equal(this.doc, tag);
    } else {
      this.doc = tag;
    }
  }
}

let devices = new NamedSet<Device>();

function withContext<T>(context: string, fn: () => T) {
  Object.defineProperty(fn, 'name', { value: context });
  return fn();
}

function handleIntFormat(format?: string): RustType {
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
  private readonly baseKind: 'Request' | 'Response';
  private readonly device: Device;

  constructor(baseKind: 'Request' | 'Response', device: Device) {
    this.baseKind = baseKind;
    this.device = device;
  }

  handleType(
    name: string,
    schema: SchemaObject | ReferenceObject = err('Missing schema'),
  ): RustType {
    return withContext(name, () => {
      ({ name = name, target: schema } = nameAndTarget(schema));
      switch (schema.type) {
        case 'integer':
          if (schema.oneOf) {
            return registerType(
              this.device,
              schema,
              (schema) => new EnumType(name, schema),
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
            return rusty(
              'std::time::SystemTime',
              `time_repr::TimeRepr<time_repr::${format}>`,
            );
          }
          return rusty('String');
        }
        case 'boolean':
          return rusty('bool');
        case 'object': {
          return registerType(
            this.device,
            schema,
            (schema) => new ObjectType(this, name, schema),
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
    schema?: SchemaObject | ReferenceObject,
    required = false,
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
      | ReferenceObject = err('Missing content'),
  ): RustType {
    let { baseKind } = this;
    let name = `${this.device.name}${canonicalMethodName}${baseKind}`;
    return withContext(name, () => {
      ({ name = name, target: body } = nameAndTarget(body));
      let doc = getDoc(body);
      let {
        [contentType]: { schema = err('Missing schema') } = err(
          `Missing ${contentType}`,
        ),
        ...otherContentTypes
      } = body.content ?? err('Missing content');
      assertEmpty(otherContentTypes, 'Unexpected types');
      let baseRef = `#/components/schemas/Alpaca${baseKind}`;
      if ((schema as ReferenceObject).$ref === baseRef) {
        return rusty('()');
      }
      ({ name = name, target: schema } = nameAndTarget(schema));
      if (name.endsWith(baseKind)) {
        name = name.slice(0, -baseKind.length);
      }
      return registerType(this.device, schema, (schema) => {
        if (name === 'ImageArray') {
          return rusty('ImageArray');
        }

        doc = getDoc(schema) ?? doc;
        let {
          allOf: [base, extension, ...otherItemsInAllOf] = err('Missing allOf'),
          ...otherPropsInSchema
        } = schema;
        assert.deepEqual(otherItemsInAllOf, [], 'Unexpected items in allOf');
        assertEmpty(
          otherPropsInSchema,
          'Unexpected properties in content schema',
        );
        assert.equal((base as ReferenceObject).$ref, baseRef);
        assert.ok(extension && !('$ref' in extension));
        let { properties, required, ...otherPropsInExtension } = extension;
        assertEmpty(
          otherPropsInExtension,
          'Unexpected properties in extension',
        );
        // Special-case value responses.
        if (
          baseKind === 'Response' &&
          properties !== undefined &&
          isDeepStrictEqual(Object.keys(properties), ['Value'])
        ) {
          let valueType = this.handleType(name, properties.Value);
          return rusty(valueType.toString(), valueType.convertVia);
        }

        const ctor = baseKind === 'Request' ? RequestType : ObjectType;

        return new ctor(this, name, {
          properties,
          required,
          description: doc,
        });
      });
    });
  }
}

function extractParams(
  methodSchema: OpenAPIV3_1.OperationObject,
  device: Device,
  extraExpectedParams: string[],
) {
  let params = (methodSchema.parameters ?? err('Missing parameters')).slice();
  let expectedParams = [...extraExpectedParams, 'device_number'];
  if (device.isBaseDevice) {
    expectedParams.push('device_type');
  }
  for (let expectedParam of expectedParams) {
    let param = params.findIndex(
      (param) =>
        (param as ReferenceObject).$ref ===
        `#/components/parameters/${expectedParam}`,
    );
    assert.ok(param !== -1, `Missing parameter ${expectedParam}`);
    params.splice(param, 1);
  }
  return params;
}

for (let [path, methods = err('Missing methods')] of Object.entries(
  api.paths ?? err('Missing paths'),
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

    withContext('Get', () => {
      if (!get) return;

      let params = extractParams(get, device, [
        'ClientIDQuery',
        'ClientTransactionIDQuery',
      ]);

      let method = new DeviceMethod(device, 'Get', methodPath, get);

      let paramCtx = new TypeContext('Request', device);

      for (let { target: param } of params.map(nameAndTarget)) {
        assert.equal(param?.in, 'query', 'Parameter is not a query parameter');
        method.resolvedArgs.add(
          new Property(
            param.name,
            paramCtx.handleOptType(
              `${device.name}${method.name}Request${param.name}`,
              param.schema,
              param.required,
            ),
            getDoc(param),
          ),
        );
      }

      device.methods.add(method);
    });

    withContext('Put', () => {
      if (!put) return;

      let params = extractParams(put, device, []);
      assert.deepEqual(params, [], 'Unexpected parameters in PUT method');

      let method = new DeviceMethod(
        device,
        get ? 'Put(Setter)' : 'Put',
        methodPath,
        put,
      );

      let argsType = new TypeContext('Request', device).handleContent(
        method.name,
        'application/x-www-form-urlencoded',
        put.requestBody,
      );

      if (!argsType.isVoid()) {
        let resolvedType = types.get(argsType.toString());
        assert.ok(
          resolvedType instanceof RequestType,
          'Registered type is not a request',
        );
        method.resolvedArgs = resolvedType.properties;
      }

      device.methods.add(method);
    });
  });
}

// Fork `interface_version` to individual traits because we want to version it separately.
{
  let baseDevice = devices.get('{device_type}')!;
  let interfaceVersionMethod = baseDevice.methods.get('interface_version')!;
  for (let device of devices.values()) {
    if (device.isBaseDevice) continue;
    device.methods.add(
      // override `device` property to point to the specific non-base device
      Object.create(interfaceVersionMethod, {
        device: { value: device },
      }),
    );
  }
  baseDevice.methods.delete('interface_version');
}

function stringifyDoc(doc = '') {
  doc = doc.trim();
  if (!doc) return '';
  return (
    doc
      /// Finish with a period.
      .replace(/(?<!\.)$/, '.')
      // Change "`InterfaceV1 Only` ...actual description" to be "actual description\n\n_InterfaceV1 Only_"
      .replace(/^`(.+?)\.?`\s*(.*)$/s, '$2\n\n_$1._')
      // If there is no summary, split out first sentence as summary.
      .replace(/^(.*?(?<!e\.g|i\.e)\.) (?=[A-Z])/, '$1\n\n')
      // Mark code blocks as text so that they're not picked up as Rust.
      .replace(/^(```)(\n.*?\n```)$/gms, '$1text$2')
      // For each line, add doc-comment markers and trim the trailing whitespace.
      .replace(/^(.*?) *(?=\n|$)/gm, '/// $1')
  );
}

let rendered = `
// DO NOT EDIT. This file is auto-generated by running 'pnpm generate' in the 'src/api/autogen' folder.

/*!
${api.info.title} ${api.info.version}

${api.info.description}
*/

#![expect(clippy::doc_markdown)]

mod devices_impl;
mod server_info;
mod time_repr;

use crate::{ASCOMError, ASCOMResult};
use crate::macros::{rpc_mod, rpc_trait};
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

rpc_mod! {${devices.toString((device) =>
  device.isBaseDevice
    ? ''
    : `
    ${device.name} = "${device.path}",`,
)}
}
`;

try {
  let rustfmt = spawnSync('rustfmt', ['--edition=2021'], {
    encoding: 'utf-8',
    input: rendered,
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
  mode: /* readonly */ 0o444,
});
