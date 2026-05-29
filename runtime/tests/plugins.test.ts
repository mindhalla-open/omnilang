import { SchemaGeneratorRegistry } from "../src/plugins/base";
import { PrismaSchemaGenerator } from "../src/plugins/prisma";
import { SqlSchemaGenerator, mapTypeToSql, generateSqlDdl } from "../src/plugins/sql";
import { SpecIR, TypeRef } from "../src/types";

function createTypeRef(name: string, typeArgs: TypeRef[] = []): TypeRef {
  return {
    name,
    type_args: typeArgs,
    union_members: [],
    intersection_members: [],
    span: { start: 0, end: 0 }
  };
}

describe("Schema Generator Registry", () => {
  beforeAll(() => {
    SchemaGeneratorRegistry.clear();
    SchemaGeneratorRegistry.register(new PrismaSchemaGenerator());
    SchemaGeneratorRegistry.register(new SqlSchemaGenerator());
  });

  test("should resolve target with explicit provider prefix", () => {
    const prismaPlugin = SchemaGeneratorRegistry.getPluginForTarget("prisma:postgresql");
    expect(prismaPlugin).toBeDefined();
    expect(prismaPlugin?.name).toBe("prisma");

    const sqlPlugin = SchemaGeneratorRegistry.getPluginForTarget("sql:postgresql");
    expect(sqlPlugin).toBeDefined();
    expect(sqlPlugin?.name).toBe("sql");
  });

  test("should resolve target using supports()", () => {
    const plugin = SchemaGeneratorRegistry.getPluginForTarget("sql");
    expect(plugin?.name).toBe("sql");
  });

  test("should fallback to default (first registered) when unknown target is provided", () => {
    const plugin = SchemaGeneratorRegistry.getPluginForTarget("unknown_db");
    expect(plugin?.name).toBe("prisma");
  });
});

describe("SQL Generator - mapTypeToSql", () => {
  test("should map primitive types to correct SQL types", () => {
    expect(mapTypeToSql(createTypeRef("UUID"))).toBe("VARCHAR(36)");
    expect(mapTypeToSql(createTypeRef("String"))).toBe("VARCHAR(255)");
    expect(mapTypeToSql(createTypeRef("Int"))).toBe("INTEGER");
    expect(mapTypeToSql(createTypeRef("Float"))).toBe("DOUBLE PRECISION");
    expect(mapTypeToSql(createTypeRef("Boolean"))).toBe("BOOLEAN");
    expect(mapTypeToSql(createTypeRef("DateTime"))).toBe("TIMESTAMP");
    expect(mapTypeToSql(createTypeRef("Decimal"))).toBe("NUMERIC");
    expect(mapTypeToSql(createTypeRef("Unknown"))).toBe("VARCHAR(255)");
  });

  test("should resolve type aliases recursively in SQL mapping", () => {
    const ir: SpecIR = {
      ir_version: "1",
      module_path: ["test"],
      source_file: {
        module: { path: ["test"], span: { start: 0, end: 0 } },
        imports: [],
        exports: [],
        declarations: [
          {
            Type: {
              name: "CustomId",
              type_params: [],
              kind: { Alias: createTypeRef("UUID") },
              visibility: "Public",
              doc_comment: null,
              span: { start: 0, end: 0 }
            }
          }
        ]
      },
      types: [],
      services: [],
      build_order: [],
      type_mappings: [],
      stats: {
        type_count: 1,
        service_count: 0,
        operation_count: 0,
        test_count: 0,
        constraint_count: 0,
        metric_count: 0,
        component_count: 0,
        pipeline_count: 0,
        workflow_count: 0,
        agent_count: 0,
        schema_count: 0,
        policy_count: 0
      }
    };
    expect(mapTypeToSql(createTypeRef("CustomId"), ir)).toBe("VARCHAR(36)");
  });
});

describe("SQL Generator - generateSqlDdl", () => {
  test("should generate valid DDL SQL script", () => {
    const ir: SpecIR = {
      ir_version: "1",
      module_path: ["test"],
      source_file: {
        module: { path: ["test"], span: { start: 0, end: 0 } },
        imports: [],
        exports: [],
        declarations: [
          {
            Schema: {
              name: "StoreDB",
              goal: "Manage products and reviews",
              target: "sql:postgresql",
              entities: [
                {
                  name: "Product",
                  fields: [
                    {
                      name: "id",
                      ty: createTypeRef("Int"),
                      default: null,
                      decorators: [{ name: "primary", args: [], span: { start: 0, end: 0 } }],
                      doc_comment: null,
                      span: { start: 0, end: 0 }
                    },
                    {
                      name: "title",
                      ty: createTypeRef("String"),
                      default: null,
                      decorators: [],
                      doc_comment: null,
                      span: { start: 0, end: 0 }
                    }
                  ],
                  doc_comment: null,
                  span: { start: 0, end: 0 }
                },
                {
                  name: "Review",
                  fields: [
                    {
                      name: "id",
                      ty: createTypeRef("UUID"),
                      default: null,
                      decorators: [{ name: "primary", args: [], span: { start: 0, end: 0 } }],
                      doc_comment: null,
                      span: { start: 0, end: 0 }
                    },
                    {
                      name: "rating",
                      ty: createTypeRef("Int"),
                      default: null,
                      decorators: [{ name: "indexed", args: [], span: { start: 0, end: 0 } }],
                      doc_comment: null,
                      span: { start: 0, end: 0 }
                    }
                  ],
                  doc_comment: null,
                  span: { start: 0, end: 0 }
                }
              ],
              relations: [
                {
                  lhs: "Product",
                  rel_type: "has_many",
                  rhs: "Review",
                  args: [],
                  span: { start: 0, end: 0 }
                }
              ],
              indexes: [
                {
                  entity: "Review",
                  fields: ["id", "rating"],
                  where: null,
                  span: { start: 0, end: 0 }
                }
              ],
              constraints: [],
              span: { start: 0, end: 0 }
            }
          }
        ]
      },
      types: [],
      services: [],
      build_order: [],
      type_mappings: [],
      stats: {
        type_count: 0,
        service_count: 0,
        operation_count: 0,
        test_count: 0,
        constraint_count: 0,
        metric_count: 0,
        component_count: 0,
        pipeline_count: 0,
        workflow_count: 0,
        agent_count: 0,
        schema_count: 1,
        policy_count: 0
      }
    };

    const sqlStr = generateSqlDdl(ir);
    expect(sqlStr).not.toBeNull();
    expect(sqlStr).toContain("CREATE TABLE Product (");
    expect(sqlStr).toContain("id              INTEGER PRIMARY KEY");
    expect(sqlStr).toContain("title           VARCHAR(255) NOT NULL");
    
    expect(sqlStr).toContain("CREATE TABLE Review (");
    expect(sqlStr).toContain("id              VARCHAR(36) PRIMARY KEY");
    expect(sqlStr).toContain("rating          INTEGER NOT NULL");
    expect(sqlStr).toContain("productId       INTEGER"); // injected FK column

    expect(sqlStr).toContain("CREATE INDEX idx_review_rating ON Review(rating);");
    expect(sqlStr).toContain("CREATE INDEX idx_review_id_rating ON Review(id, rating);");
    expect(sqlStr).toContain("ALTER TABLE Review ADD CONSTRAINT fk_review_product FOREIGN KEY (productId) REFERENCES Product(id);");
  });
});
