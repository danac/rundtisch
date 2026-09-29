type JsonDescriptor = {
  type: PublicKeyCredentialType
  id: string
  transports?: AuthenticatorTransport[]
}

type CreationPublicKey = {
  rp: PublicKeyCredentialRpEntity
  user: { id: string; name: string; displayName: string }
  challenge: string
  pubKeyCredParams: PublicKeyCredentialParameters[]
  timeout?: number
  excludeCredentials?: JsonDescriptor[]
  authenticatorSelection?: AuthenticatorSelectionCriteria
  attestation?: AttestationConveyancePreference
  extensions?: AuthenticationExtensionsClientInputs
}

type RequestPublicKey = {
  challenge: string
  timeout?: number
  rpId?: string
  allowCredentials?: JsonDescriptor[]
  userVerification?: UserVerificationRequirement
  extensions?: AuthenticationExtensionsClientInputs
}

function bytesToBase64Url(bytes: Uint8Array): string {
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/u, '')
}

function base64UrlToBytes(value: string): Uint8Array<ArrayBuffer> {
  const padded =
    value.replaceAll('-', '+').replaceAll('_', '/') + '='.repeat((4 - (value.length % 4)) % 4)
  const binary = atob(padded)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index)
  }
  return bytes
}

function asPublicKeyCredential(credential: Credential | null): PublicKeyCredential {
  if (!(credential instanceof PublicKeyCredential)) {
    throw new Error('passkey_cancelled')
  }
  return credential
}

export function passkeySupported(): boolean {
  return typeof PublicKeyCredential !== 'undefined'
}

export async function conditionalMediationAvailable(): Promise<boolean> {
  if (
    !passkeySupported() ||
    typeof PublicKeyCredential.isConditionalMediationAvailable !== 'function'
  ) {
    return false
  }
  try {
    return await PublicKeyCredential.isConditionalMediationAvailable()
  } catch {
    return false
  }
}

export async function createPasskey(options: unknown): Promise<unknown> {
  const { publicKey } = options as { publicKey: CreationPublicKey }
  const credential = asPublicKeyCredential(
    await navigator.credentials.create({
      publicKey: {
        rp: publicKey.rp,
        user: { ...publicKey.user, id: base64UrlToBytes(publicKey.user.id) },
        challenge: base64UrlToBytes(publicKey.challenge),
        pubKeyCredParams: publicKey.pubKeyCredParams,
        timeout: publicKey.timeout,
        excludeCredentials: publicKey.excludeCredentials?.map((item) => ({
          type: item.type,
          id: base64UrlToBytes(item.id),
          transports: item.transports,
        })),
        authenticatorSelection: publicKey.authenticatorSelection,
        attestation: publicKey.attestation,
        extensions: publicKey.extensions,
      },
    }),
  )
  const response = credential.response
  if (!(response instanceof AuthenticatorAttestationResponse)) {
    throw new Error('passkey_cancelled')
  }
  return {
    id: credential.id,
    rawId: bytesToBase64Url(new Uint8Array(credential.rawId)),
    type: credential.type,
    response: {
      attestationObject: bytesToBase64Url(new Uint8Array(response.attestationObject)),
      clientDataJSON: bytesToBase64Url(new Uint8Array(response.clientDataJSON)),
      transports: response.getTransports(),
    },
  }
}

export async function getPasskey(
  options: unknown,
  extra?: { mediation?: CredentialMediationRequirement; signal?: AbortSignal },
): Promise<unknown> {
  const { publicKey } = options as { publicKey: RequestPublicKey }
  const allowCredentials = publicKey.allowCredentials?.length
    ? publicKey.allowCredentials.map((item) => ({
        type: item.type,
        id: base64UrlToBytes(item.id),
        transports: item.transports,
      }))
    : undefined
  const credential = asPublicKeyCredential(
    await navigator.credentials.get({
      mediation: extra?.mediation,
      signal: extra?.signal,
      publicKey: {
        challenge: base64UrlToBytes(publicKey.challenge),
        timeout: publicKey.timeout,
        rpId: publicKey.rpId,
        allowCredentials,
        userVerification: publicKey.userVerification,
        extensions: publicKey.extensions,
      },
    }),
  )
  const response = credential.response
  if (!(response instanceof AuthenticatorAssertionResponse)) {
    throw new Error('passkey_cancelled')
  }
  return {
    id: credential.id,
    rawId: bytesToBase64Url(new Uint8Array(credential.rawId)),
    type: credential.type,
    response: {
      authenticatorData: bytesToBase64Url(new Uint8Array(response.authenticatorData)),
      clientDataJSON: bytesToBase64Url(new Uint8Array(response.clientDataJSON)),
      signature: bytesToBase64Url(new Uint8Array(response.signature)),
      userHandle: response.userHandle
        ? bytesToBase64Url(new Uint8Array(response.userHandle))
        : null,
    },
  }
}
