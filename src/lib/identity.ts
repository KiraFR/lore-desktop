/** Pure display helpers for the signed-in identity (avatar initials, labels). */

function localPart(email: string | null | undefined): string {
  return email?.split('@')[0] ?? ''
}

/** Initials for the avatar: display name first, else the email local part. */
export function initialsFor(displayName: string | null | undefined, email: string | null | undefined): string {
  const src = displayName?.trim() || localPart(email)
  if (!src) return '?'
  const words = src.split(/[\s._-]+/).filter(Boolean)
  if (words.length >= 2) return (words[0][0] + words[1][0]).toUpperCase()
  return src.slice(0, 2).toUpperCase()
}

/** Human label for the identity: display name, else email local part. */
export function displayNameFor(displayName: string | null | undefined, email: string | null | undefined): string {
  return displayName?.trim() || localPart(email) || 'Not signed in'
}
