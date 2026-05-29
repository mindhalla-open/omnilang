import { mapTypeToPrisma, injectRelationFields, generatePrismaSchema, PrismaSchemaDecl } from "../src/plugins/prisma";
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

describe("Prisma Generator - mapTypeToPrisma", () => {
  test("should map basic types correctly", () => {
    expect(mapTypeToPrisma(createTypeRef("UUID"))).toBe("String");
    expect(mapTypeToPrisma(createTypeRef("String"))).toBe("String");
    expect(mapTypeToPrisma(createTypeRef("Int"))).toBe("Int");
    expect(mapTypeToPrisma(createTypeRef("Float"))).toBe("Float");
    expect(mapTypeToPrisma(createTypeRef("Boolean"))).toBe("Boolean");
    expect(mapTypeToPrisma(createTypeRef("DateTime"))).toBe("DateTime");
    expect(mapTypeToPrisma(createTypeRef("Decimal"))).toBe("Decimal");
    expect(mapTypeToPrisma(createTypeRef("UserId"))).toBe("UserId"); // custom type fallback
  });

  test("should map option types correctly", () => {
    const optionInt = createTypeRef("Option", [createTypeRef("Int")]);
    expect(mapTypeToPrisma(optionInt)).toBe("Int?");
  });

  test("should map list types correctly", () => {
    const listUuid = createTypeRef("List", [createTypeRef("UUID")]);
    expect(mapTypeToPrisma(listUuid)).toBe("String[]");
  });

  test("should map nested list of options correctly", () => {
    const listOptionInt = createTypeRef("List", [createTypeRef("Option", [createTypeRef("Int")])]);
    expect(mapTypeToPrisma(listOptionInt)).toBe("Int?[]");
  });

  test("should resolve type aliases recursively", () => {
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
              name: "UserId",
              type_params: [],
              kind: { Alias: createTypeRef("String") },
              visibility: "Public",
              doc_comment: null,
              span: { start: 0, end: 0 }
            }
          },
          {
            Type: {
              name: "AccountId",
              type_params: [],
              kind: { Alias: createTypeRef("UserId") },
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
        type_count: 2,
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

    expect(mapTypeToPrisma(createTypeRef("UserId"), ir)).toBe("String");
    expect(mapTypeToPrisma(createTypeRef("AccountId"), ir)).toBe("String");
  });
});

describe("Prisma Generator - injectRelationFields", () => {
  test("should expand has_many relations correctly", () => {
    const schema: PrismaSchemaDecl = {
      name: "TestDB",
      goal: "Test has_many",
      target: "postgresql",
      entities: [
        {
          name: "User",
          fields: [
            {
              name: "id",
              ty: createTypeRef("UUID"),
              default: null,
              decorators: [{ name: "primary", args: [], span: { start: 0, end: 0 } }],
              doc_comment: null,
              span: { start: 0, end: 0 }
            }
          ],
          doc_comment: null,
          span: { start: 0, end: 0 }
        },
        {
          name: "Post",
          fields: [
            {
              name: "id",
              ty: createTypeRef("Int"),
              default: null,
              decorators: [{ name: "primary", args: [], span: { start: 0, end: 0 } }],
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
          lhs: "User",
          rel_type: "has_many",
          rhs: "Post",
          args: [],
          span: { start: 0, end: 0 }
        }
      ],
      indexes: [],
      constraints: [],
      span: { start: 0, end: 0 }
    };

    injectRelationFields(schema);

    // User should have `posts: Post[]`
    const user = schema.entities.find(e => e.name === "User")!;
    const postsField = user.fields.find(f => f.name === "posts")!;
    expect(postsField).toBeDefined();
    expect(postsField.ty.name).toBe("List");
    expect(postsField.ty.type_args[0].name).toBe("Post");

    // Post should have `user: User` and `userId: String`
    const post = schema.entities.find(e => e.name === "Post")!;
    const userField = post.fields.find(f => f.name === "user")!;
    expect(userField).toBeDefined();
    expect(userField.ty.name).toBe("User");
    expect(userField.relationConfig).toBeDefined();
    expect(userField.relationConfig?.fields).toEqual(["userId"]);
    expect(userField.relationConfig?.references).toEqual(["id"]);

    const userIdField = post.fields.find(f => f.name === "userId")!;
    expect(userIdField).toBeDefined();
    expect(userIdField.ty.name).toBe("UUID"); // mirrors User's id type
  });

  test("should expand has_one relations correctly", () => {
    const schema: PrismaSchemaDecl = {
      name: "TestDB",
      goal: "Test has_one",
      target: "postgresql",
      entities: [
        {
          name: "User",
          fields: [
            {
              name: "id",
              ty: createTypeRef("UUID"),
              default: null,
              decorators: [{ name: "primary", args: [], span: { start: 0, end: 0 } }],
              doc_comment: null,
              span: { start: 0, end: 0 }
            }
          ],
          doc_comment: null,
          span: { start: 0, end: 0 }
        },
        {
          name: "Profile",
          fields: [
            {
              name: "id",
              ty: createTypeRef("Int"),
              default: null,
              decorators: [{ name: "primary", args: [], span: { start: 0, end: 0 } }],
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
          lhs: "User",
          rel_type: "has_one",
          rhs: "Profile",
          args: [],
          span: { start: 0, end: 0 }
        }
      ],
      indexes: [],
      constraints: [],
      span: { start: 0, end: 0 }
    };

    injectRelationFields(schema);

    // User should have `profile: Profile?`
    const user = schema.entities.find(e => e.name === "User")!;
    const profileField = user.fields.find(f => f.name === "profile")!;
    expect(profileField).toBeDefined();
    expect(profileField.ty.name).toBe("Option");
    expect(profileField.ty.type_args[0].name).toBe("Profile");

    // Profile should have `user: User` and `userId: String @unique`
    const profile = schema.entities.find(e => e.name === "Profile")!;
    const userField = profile.fields.find(f => f.name === "user")!;
    expect(userField).toBeDefined();
    expect(userField.ty.name).toBe("User");
    expect(userField.relationConfig).toBeDefined();

    const userIdField = profile.fields.find(f => f.name === "userId")!;
    expect(userIdField).toBeDefined();
    expect(userIdField.decorators.some(d => d.name === "unique")).toBe(true);
  });
});

describe("Prisma Generator - generatePrismaSchema", () => {
  test("should return null if no schema block is present in SpecIR", () => {
    const ir: SpecIR = {
      ir_version: "1",
      module_path: ["test"],
      source_file: {
        module: { path: ["test"], span: { start: 0, end: 0 } },
        imports: [],
        exports: [],
        declarations: []
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
        schema_count: 0,
        policy_count: 0
      }
    };
    expect(generatePrismaSchema(ir)).toBeNull();
  });

  test("should output valid prisma schema content when schema block is present", () => {
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
              name: "TestDB",
              goal: "Test database",
              target: "mysql",
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
                      name: "sku",
                      ty: createTypeRef("String"),
                      default: null,
                      decorators: [{ name: "unique", args: [], span: { start: 0, end: 0 } }],
                      doc_comment: null,
                      span: { start: 0, end: 0 }
                    },
                    {
                      name: "price",
                      ty: createTypeRef("Decimal"),
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
              relations: [],
              indexes: [
                {
                  entity: "Product",
                  fields: ["sku", "price"],
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

    const schemaStr = generatePrismaSchema(ir);
    expect(schemaStr).not.toBeNull();
    expect(schemaStr).toContain('provider = "mysql"');
    expect(schemaStr).toContain("model Product {");
    expect(schemaStr).toContain("id           Int @id @default(autoincrement())");
    expect(schemaStr).toContain("sku          String @unique");
    expect(schemaStr).toContain("price        Decimal");
    expect(schemaStr).toContain("@@index([price])");
    expect(schemaStr).toContain("@@index([sku, price])");
  });
});
