import Foundation
struct LocalFailure: Error { let code: String }
@main struct Validate {
 static func main() {
  do {
   let data=FileHandle.standardInput.readDataToEndOfFile()
   let value=try parseModelJSONObject(String(decoding:data,as:UTF8.self))
   let valid=try validateCategoryPlan(value)
   FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject:valid,options:[.sortedKeys]))
  } catch { exit(1) }
 }
}
