// This first stratum can run offline, or read one public-to-members post in a
// disposable local Snapback dev store. The record ID is a public build setting.
const recordId = import.meta.env.ANSUZ_READING_ID ?? "";
const localQuestion = "What is ready to take root?";

export async function loadReading(origin: string) {
  if (!recordId) {
    return { question: localQuestion, id: "ansuz:007:seed", sourceLabel: "Sample reading" };
  }

  if (!/^http:\/\/127\.0\.0\.1:[0-9]+$/.test(origin)) {
    throw new Error("The shared preview requires a loopback development store.");
  }
  const response = await fetch(origin + "/q/postById", {
    method: "POST",
    headers: { "content-type": "application/json", "x-snapback-persona": "reader" },
    body: JSON.stringify({ args: { id: recordId } }),
  });
  if (!response.ok) throw new Error("The shared reading is unavailable.");
  const result = await response.json();
  const post = result.data;
  if (result.complete !== true || !post || post.deleted || post.id !== recordId ||
      post.visibility !== "public" || typeof post.body !== "string" || !post.body.trim()) {
    throw new Error("The shared reading is unavailable.");
  }
  return { question: post.body, id: post.id, sourceLabel: "Shared Jera post · Local preview" };
}
