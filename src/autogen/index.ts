import openapi from '@readme/openapi-parser';
import { readFile, writeFile } from 'fs/promises';
import { render } from 'ejs';
import { spawnSync } from 'child_process';
import { toSnakeCase, toPascalCase } from 'js-convert-case';
import { OpenAPIV3 } from 'openapi-types';
import * as assert from 'assert/strict';
import { getCanonicalNames } from './xml-names.js';

let api = (await openapi.parse(
  './AlpacaDeviceAPI_v1.yaml'
)) as OpenAPIV3.Document;
let refs = await openapi.resolve(api);
let template = await readFile('./server.ejs', 'utf-8');

let path2id = Object.fromEntries(
  Object.keys(api.paths || {}).map(path => [
    path,
    path
      .split('/')
      .slice(1)
      .filter(x => !/^\{.*\}$/.test(x))
      .join('_')
  ])
);

function* ops() {
  for (let [path, methods = {}] of Object.entries(api.paths)) {
    let pathId = path2id[path];
    for (let method of Object.values(OpenAPIV3.HttpMethods)) {
      let operation = methods[method];
      if (operation) {
        yield { path, method, id: `${method}_${pathId}`, operation };
      }
    }
  }
}

function isRef(maybeRef: any): maybeRef is OpenAPIV3.ReferenceObject {
  return maybeRef != null && '$ref' in maybeRef;
}

function resolveMaybeRef<T>(ref: T | OpenAPIV3.ReferenceObject): T {
  if (isRef(ref)) {
    return refs.get(ref.$ref);
  }
  return ref;
}

function getContent(
  owner: OpenAPIV3.RequestBodyObject | OpenAPIV3.ResponseObject,
  contentType: string
) {
  let { content = {} } = owner;
  let keys = Object.keys(content);
  assert.deepEqual(keys, [contentType], `Unexpected content types: ${keys}`);
  let { schema } = content[contentType];
  schema = resolveMaybeRef(schema);
  assert.equal(schema?.type, 'object', 'Content is not an object');
  return { schema: schema, content: content[contentType] };
}

function cleanupSchema(obj: OpenAPIV3.SchemaObject) {
  switch (obj.type) {
    case 'array': {
      if (!isRef(obj.items)) {
        cleanupSchema(obj.items);
      }
      return;
    }
    case 'object': {
      for (let v of Object.values(obj.properties!)) {
        if (!isRef(v)) {
          cleanupSchema(v);
        }
      }
      return;
    }
    case 'string': {
      if (obj.default === '') {
        delete obj.default;
      }
      return;
    }
    case 'boolean': {
      if (obj.default === false) {
        delete obj.default;
      }
      return;
    }
    case 'integer': {
      obj.format ??= 'int32';
      let range = {
        int32: { min: -2147483648, max: 2147483647 },
        uint32: { min: 0, max: 4294967295 }
      }[obj.format];
      if (range) {
        if (obj.minimum === range.min) {
          delete obj.minimum;
        }
        if (obj.maximum === range.max) {
          delete obj.maximum;
        }
      }
      // fallthrough
    }
    case 'number': {
      if (obj.default === 0) {
        delete obj.default;
      }
      return;
    }
  }
}

function jsonWithoutOptFields(obj: any): string {
  return JSON.stringify(obj, (k, v) => (k === 'description' ? undefined : v));
}

function withoutOptFields<T>(obj: T): T {
  return JSON.parse(jsonWithoutOptFields(obj));
}

function setXKind(schema: OpenAPIV3.SchemaObject, kind: string) {
  let prev = (schema as any)['x-kind'];
  if (prev) {
    assert.equal(prev, kind, `Conflicting x-kind: ${prev} vs ${kind}`);
  }
  (schema as any)['x-kind'] = kind;
}

function registerSchema(
  name: string,
  schema: OpenAPIV3.SchemaObject
): OpenAPIV3.ReferenceObject {
  api.components!.schemas![name] = schema;
  return { $ref: `#/components/schemas/${name}` };
}

const rustKeywords = new Set([
  'as',
  'use',
  'extern crate',
  'break',
  'const',
  'continue',
  'crate',
  'else',
  'if',
  'enum',
  'extern',
  'false',
  'fn',
  'for',
  'if',
  'impl',
  'in',
  'for',
  'let',
  'loop',
  'match',
  'mod',
  'move',
  'mut',
  'pub',
  'impl',
  'ref',
  'return',
  'Self',
  'self',
  'static',
  'struct',
  'super',
  'trait',
  'true',
  'type',
  'unsafe',
  'use',
  'where',
  'while',
  'abstract',
  'alignof',
  'become',
  'box',
  'do',
  'final',
  'macro',
  'offsetof',
  'override',
  'priv',
  'proc',
  'pure',
  'sizeof',
  'typeof',
  'unsized',
  'virtual',
  'yield'
]);

function toPropName(name: string) {
  name = toSnakeCase(name);
  if (rustKeywords.has(name)) {
    name = `r#${name}`;
  }
  return name;
}

let canonicalNames = await getCanonicalNames();

// for (let [groupPath, group] of Object.entries(groupedOps)) {
//   let canonicalGroup = canonicalNames[groupPath];
//   if (!canonicalGroup) {
//     console.warn(`Couldn't find canonical name for ${groupPath}`);
//     continue;
//   }
//   group.typeName = canonicalGroup.name;
//   for (let path of group.paths) {
//     let canonicalMethodName = canonicalGroup.methods[path.subPath];
//     if (!canonicalMethodName) {
//       console.warn(
//         `Couldn't find canonical name for ${group.typeName}::${path.subPath}`
//       );
//       continue;
//     }

//     if (path.method === 'get') {
//       if (canonicalMethodName.startsWith('Get')) {
//         canonicalMethodName = canonicalMethodName.slice(3);
//       }
//     } else if (
//       api.paths[`/${groupPath}/{device_number}/${path.subPath}`]?.get
//     ) {
//       canonicalMethodName = `Set${canonicalMethodName}`;
//     }

//     path.fnName = toPropName(canonicalMethodName);
//   }
// }

let groupedOps: Record<
  string,
  {
    typeName: string;
    description: string;
    paths: Array<{
      subPath: string;
      method: OpenAPIV3.HttpMethods;
      fnName: string;
      operation: OpenAPIV3.OperationObject;
      request: OpenAPIV3.ReferenceObject;
      response: OpenAPIV3.ReferenceObject;
    }>;
  }
> = {};

// Extract path parameters, query parameters and bodies that are not in api.components yet.
// Add all of them to api.components, and replace the originals with $ref references.
for (let { method, path, id, operation } of ops()) {
  assert.equal(operation.tags?.length, 1, 'Unexpected number of tags');
  let groupDescription = operation.tags[0];

  let pathMatch = path.match(
    /^\/(\{device_type\}|\w+)\/\{device_number\}\/(\w+)$/
  );
  assert.ok(pathMatch, `Path in unexpected format: ${path}`);
  let [, groupPath, subPath] = pathMatch;

  let groupedParams: Record<string, OpenAPIV3.ParameterObject[]> = {};
  for (let param of operation.parameters || []) {
    param = resolveMaybeRef(param);
    (groupedParams[param.in] ??= []).push(param);
  }

  let typeId = toPascalCase(id);

  let { path: pathParams, query: queryParams, ...other } = groupedParams;
  assert.deepEqual(other, {}, 'Unexpected parameters');

  assert.deepEqual(
    Object.fromEntries(
      pathParams.map(({ name, required, schema }) => [
        name,
        { type: resolveMaybeRef(schema)?.type, required }
      ])
    ),
    {
      ...(groupPath === '{device_type}'
        ? { device_type: { type: 'string', required: true } }
        : {}),
      device_number: { type: 'integer', required: true }
    },
    `Unexpected path parameters for ${method} ${path}`
  );

  let requestBody = resolveMaybeRef(operation.requestBody);

  let requestParams: OpenAPIV3.SchemaObject;
  let requestParamsRef: OpenAPIV3.ReferenceObject;

  if (method === 'get') {
    assert.ok(queryParams, `Missing query parameters for ${method} ${path}`);
    assert.ok(!requestBody, `Unexpected request body for ${method} ${path}`);

    requestParams = {
      type: 'object',
      properties: {}
    };
    for (let param of queryParams) {
      requestParams.properties![param.name] = {
        description: param.description,
        ...param.schema
      };
      if (param.required) {
        (requestParams.required ??= []).push(param.name);
      }
    }

    requestParamsRef = registerSchema(`${typeId}Request`, requestParams);
  } else {
    assert.ok(
      !queryParams,
      `Unexpected query parameters for ${method} ${path}`
    );
    assert.ok(requestBody, `Missing request body for ${method} ${path}`);

    let { content, schema } = getContent(
      requestBody,
      'application/x-www-form-urlencoded'
    );

    if (!isRef(content.schema)) {
      content.schema = registerSchema(`${typeId}Request`, schema);
    }

    requestParams = schema;
    requestParamsRef = content.schema;
  }

  setXKind(requestParams, 'Request');

  let { 200: successfulResponse, ...errorResponses } = operation.responses;

  assert.ok(
    successfulResponse,
    `Missing successful response for ${method} ${path}`
  );

  let errorResponseShape: Partial<OpenAPIV3.ResponseObject> = {
    content: {
      'text/plain': {
        schema: {
          type: 'string'
        }
      }
    }
  };
  assert.deepEqual(withoutOptFields(errorResponses), {
    400: errorResponseShape,
    500: errorResponseShape
  });

  successfulResponse = resolveMaybeRef(successfulResponse);

  let { content: responseContent, schema: responseSchema } = getContent(
    successfulResponse,
    'application/json'
  );

  if (!isRef(responseContent.schema)) {
    responseContent.schema = registerSchema(
      `${typeId}Response`,
      responseSchema
    );
  }

  setXKind(responseSchema, 'Response');

  let canonicalDevice = canonicalNames.getDevice(groupPath);

  let fnName = canonicalDevice.getMethod(subPath);
  if (method === 'get') {
    if (fnName.startsWith('Get')) {
      fnName = fnName.slice(3);
    }
  } else if (api.paths[path]?.get) {
    fnName = `Set${fnName}`;
  }
  fnName = toPropName(fnName);

  (groupedOps[groupPath] ??= {
    typeName: canonicalDevice.name,
    description: groupDescription,
    paths: []
  }).paths.push({
    subPath,
    method,
    fnName,
    operation,
    request: requestParamsRef,
    response: responseContent.schema
  });
}

let refReplacements: Record<
  string,
  OpenAPIV3.ReferenceObject | OpenAPIV3.SchemaObject
> = {};
let groupCounts: Record<string, number> = {};

// Extract enums from properties.
for (let [schemaName, schema] of Object.entries(api.components!.schemas!)) {
  schema = resolveMaybeRef(schema);
  if (schema.type === 'object') {
    for (let [propName, prop] of Object.entries(schema.properties!)) {
      if (isRef(prop)) continue;
      if (prop.enum) {
        assert.equal(prop.type, 'integer');
        schema.properties![propName] = registerSchema(
          schemaName + propName,
          prop
        );
      }
    }
  }
}

for (let [schemaName, schema] of Object.entries(api.components!.schemas!)) {
  schema = resolveMaybeRef(schema);
  cleanupSchema(schema);

  if (schema.type !== 'object') {
    continue;
  }

  let kind = (schema as any)['x-kind'];
  switch (kind) {
    case 'Response': {
      let {
        ClientTransactionID,
        ServerTransactionID,
        ErrorNumber,
        ErrorMessage,
        ...otherProperties
      } = schema.properties!;

      assert.deepEqual(
        withoutOptFields({
          ClientTransactionID,
          ServerTransactionID,
          ErrorNumber,
          ErrorMessage
        }),
        {
          ClientTransactionID: {
            type: 'integer',
            format: 'uint32'
          },
          ServerTransactionID: {
            type: 'integer',
            format: 'uint32'
          },
          ErrorNumber: {
            type: 'integer',
            format: 'int32'
          },
          ErrorMessage: {
            type: 'string'
          }
        },
        `Missing response properties in ${schemaName}`
      );

      schema.properties = otherProperties;

      if (Object.keys(otherProperties).join(',') === 'Value') {
        // Not using setXKind as I want to explicitly override it.
        (schema as any)['x-kind'] = 'ValueResponse';

        refReplacements[`#/components/schemas/${schemaName}`] =
          schema.properties.Value;

        continue;
      }

      break;
    }
    case 'Request': {
      let { ClientID, ClientTransactionID, ...otherProperties } =
        schema.properties!;

      // These defaults are missing in some definitions.
      // Ignore them for comparison purposes.
      delete (ClientID as OpenAPIV3.SchemaObject).default;
      delete (ClientTransactionID as OpenAPIV3.SchemaObject).default;

      assert.deepEqual(
        withoutOptFields({ ClientID, ClientTransactionID }),
        {
          ClientID: {
            type: 'integer',
            format: 'uint32'
          },
          ClientTransactionID: {
            type: 'integer',
            format: 'uint32'
          }
        },
        `Missing request properties in ${schemaName}`
      );

      schema.properties = otherProperties;
      break;
    }
  }

  if (Object.keys(schema.properties!).length === 0) {
    // Explicit override.
    (schema as any)['x-kind'] = 'Empty';
    refReplacements[`#/components/schemas/${schemaName}`] = {
      $ref: 'void'
    };
    continue;
  }

  let key = jsonWithoutOptFields(schema);
  groupCounts[key] ??= 0;
  groupCounts[key]++;
}

Object.entries(groupCounts)
  .filter(([_, count]) => count > 1)
  .sort(([_, count1], [__, count2]) => count2 - count1)
  .forEach(([json, count]) => {
    console.warn(
      `Found ${count} schemas with the same shape:`,
      JSON.parse(json)
    );
  });

let rendered = render(
  template,
  {
    api,
    refs,
    refReplacements,
    groupedOps,
    assert,
    getContent,
    toTypeName: toPascalCase,
    toPropName
  },
  {
    escape: x => x
  }
);

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

await writeFile('./mod.rs', rendered);
