/**
 * `@ubjs/node@0.31.0-5` ships an `index.d.ts` declaring only
 * `UniffiNativeModule`, while its runtime also exports `resolveLibPath`,
 * `ResolveLibPathError` and `FfiType`. The N-API bindings ubrn generates use
 * all three, so without this they cannot typecheck against their own runtime.
 *
 * Upstream packaging bug; delete this file once the published types match.
 */
declare module '@ubjs/node' {
  export class UniffiNativeModule {
    static open(path: string): UniffiNativeModule;
    register(definitions: unknown): Record<string, (...args: never[]) => unknown>;
  }
  /**
   * Scalar members are opaque type descriptors; `Callback`, `Struct`,
   * `Reference` and `MutReference` build one from a name or shape.
   */
  export const FfiType: Record<string, FfiTypeDescriptor> & {
    Callback(name: string): FfiTypeDescriptor;
    Struct(name: string): FfiTypeDescriptor;
    Reference(inner: FfiTypeDescriptor): FfiTypeDescriptor;
    MutReference(inner: FfiTypeDescriptor): FfiTypeDescriptor;
  };
  export type FfiTypeDescriptor = { readonly __ffiType: unique symbol };
  export class ResolveLibPathError extends Error {}
  export function resolveLibPath(options: Record<string, unknown>): string;
  const lib: {
    UniffiNativeModule: typeof UniffiNativeModule;
    FfiType: typeof FfiType;
    ResolveLibPathError: typeof ResolveLibPathError;
    resolveLibPath: typeof resolveLibPath;
  };
  export default lib;
}
