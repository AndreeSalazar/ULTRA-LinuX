// SPDX-License-Identifier: GPL-2.0-only
// Software-only laboratory: validation callback aborts invalid calls before dispatch.
#include <vulkan/vulkan.h>
#include <atomic>
#include <array>
#include <cstdio>
#include <cstdint>
#include <cstring>
#include <cstdlib>
#include <vector>
#include "medicion.hpp"

namespace {
struct Evidence {
    std::atomic<unsigned> errors{0}, expected{0};
    std::atomic<bool> expecting_size{false};
    std::atomic<bool> expecting_dependency{false};
};
VKAPI_ATTR VkBool32 VKAPI_CALL message(VkDebugUtilsMessageSeverityFlagBitsEXT severity,
    VkDebugUtilsMessageTypeFlagsEXT, const VkDebugUtilsMessengerCallbackDataEXT* data, void* user) {
    auto& evidence = *static_cast<Evidence*>(user);
    if ((severity & VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT) != 0) {
        ++evidence.errors;
        const char* id = data->pMessageIdName ? data->pMessageIdName : "sin-VUID";
        if (evidence.expecting_size.load() &&
            std::strcmp(id, "VUID-VkBufferCreateInfo-size-00912") == 0) ++evidence.expected;
        if (evidence.expecting_dependency.load() &&
            std::strcmp(id, "SYNC-HAZARD-READ-AFTER-WRITE") == 0) ++evidence.expected;
        std::printf("VALIDACION %s\n", id);
        return VK_TRUE;
    }
    return VK_FALSE;
}
// Host-coherent allocation. Queue users must establish completion before destruction.
// Cleanup runs on allocation/binding/mapping failure, in dependency order.
struct HostBuffer {
    VkDevice device;
    VkBuffer buffer = VK_NULL_HANDLE;
    VkDeviceMemory memory = VK_NULL_HANDLE;
    void* mapped = nullptr;
    explicit HostBuffer(VkDevice owner) : device(owner) {}
    HostBuffer(const HostBuffer&) = delete;
    HostBuffer& operator=(const HostBuffer&) = delete;
    bool initialize(VkPhysicalDevice physical, VkBufferUsageFlags usage) {
        VkBufferCreateInfo info{};
        info.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
        info.size = 4096;
        info.usage = usage;
        info.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
        if (vkCreateBuffer(device, &info, nullptr, &buffer) != VK_SUCCESS) return false;
        VkMemoryRequirements requirements{};
        vkGetBufferMemoryRequirements(device, buffer, &requirements);
        if (requirements.size > 1024 * 1024) return false;
        VkPhysicalDeviceMemoryProperties types{};
        vkGetPhysicalDeviceMemoryProperties(physical, &types);
        constexpr VkMemoryPropertyFlags flags = VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT |
                                               VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
        for (std::uint32_t i = 0; i < types.memoryTypeCount; ++i) {
            if ((requirements.memoryTypeBits & (std::uint32_t{1} << i)) == 0 ||
                (types.memoryTypes[i].propertyFlags & flags) != flags) continue;
            VkMemoryAllocateInfo allocation{};
            allocation.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
            allocation.allocationSize = requirements.size;
            allocation.memoryTypeIndex = i;
            return vkAllocateMemory(device, &allocation, nullptr, &memory) == VK_SUCCESS &&
                vkBindBufferMemory(device, buffer, memory, 0) == VK_SUCCESS &&
                vkMapMemory(device, memory, 0, 4096, 0, &mapped) == VK_SUCCESS;
        }
        return false;
    }
    ~HostBuffer() {
        if (mapped) vkUnmapMemory(device, memory);
        if (buffer) vkDestroyBuffer(device, buffer, nullptr);
        if (memory) vkFreeMemory(device, memory, nullptr);
    }
};
// Never destroy or reuse resources whose completion has not been established.
// A hung software queue ends this disposable process; the outer sandbox is bounded too.
[[noreturn]] void queue_failure(const char* step, VkResult result) {
    std::fprintf(stderr, "ROJO [COLA]: %s resultado=%d; sin liberar recursos pendientes\n",
                 step, static_cast<int>(result));
    std::_Exit(1);
}
struct Transfer {
    VkDevice device;
    HostBuffer resource;
    HostBuffer destination;
    bool copy = false;
    VkCommandPool pool = VK_NULL_HANDLE;
    VkCommandBuffer command = VK_NULL_HANDLE;
    VkFence fence = VK_NULL_HANDLE;
    explicit Transfer(VkDevice owner) : device(owner), resource(owner), destination(owner) {}
    Transfer(const Transfer&) = delete;
    Transfer& operator=(const Transfer&) = delete;
    ~Transfer() {
        if (fence) vkDestroyFence(device, fence, nullptr);
        if (pool) vkDestroyCommandPool(device, pool, nullptr);
    }
    bool initialize(VkPhysicalDevice physical, std::uint32_t family, bool copy_buffer = false,
                    bool with_dependency = true) {
        copy = copy_buffer;
        VkBufferUsageFlags usage = VK_BUFFER_USAGE_TRANSFER_DST_BIT;
        if (copy) usage |= VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
        if (!resource.initialize(physical, usage)) return false;
        if (copy && !destination.initialize(physical, VK_BUFFER_USAGE_TRANSFER_DST_BIT)) return false;
        VkCommandPoolCreateInfo pool_info{};
        pool_info.sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO;
        pool_info.queueFamilyIndex = family;
        if (vkCreateCommandPool(device, &pool_info, nullptr, &pool) != VK_SUCCESS) return false;
        VkCommandBufferAllocateInfo allocation{};
        allocation.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO;
        allocation.commandPool = pool;
        allocation.level = VK_COMMAND_BUFFER_LEVEL_PRIMARY;
        allocation.commandBufferCount = 1;
        if (vkAllocateCommandBuffers(device, &allocation, &command) != VK_SUCCESS) return false;
        VkFenceCreateInfo fence_info{};
        fence_info.sType = VK_STRUCTURE_TYPE_FENCE_CREATE_INFO;
        if (vkCreateFence(device, &fence_info, nullptr, &fence) != VK_SUCCESS) return false;
        VkCommandBufferBeginInfo begin{};
        begin.sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO;
        if (vkBeginCommandBuffer(command, &begin) != VK_SUCCESS) return false;
        vkCmdFillBuffer(command, resource.buffer, 0, 4096, 0x13579BDFU);
        if (copy) {
            VkMemoryBarrier dependency{};
            dependency.sType = VK_STRUCTURE_TYPE_MEMORY_BARRIER;
            dependency.srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
            dependency.dstAccessMask = VK_ACCESS_TRANSFER_READ_BIT;
            if (with_dependency)
                vkCmdPipelineBarrier(command, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT,
                                     0, 1, &dependency, 0, nullptr, 0, nullptr);
            const VkBufferCopy region{0, 0, 4096};
            vkCmdCopyBuffer(command, resource.buffer, destination.buffer, 1, &region);
        }
        VkMemoryBarrier barrier{};
        barrier.sType = VK_STRUCTURE_TYPE_MEMORY_BARRIER;
        barrier.srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT;
        barrier.dstAccessMask = VK_ACCESS_HOST_READ_BIT;
        vkCmdPipelineBarrier(command, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_HOST_BIT,
                             0, 1, &barrier, 0, nullptr, 0, nullptr);
        return vkEndCommandBuffer(command) == VK_SUCCESS;
    }
    bool run(VkQueue queue) {
        // Poison the host view each time: stale data cannot masquerade as a successful fill.
        std::memset(resource.mapped, 0, 4096);
        if (copy) std::memset(destination.mapped, 0, 4096);
        if (vkResetFences(device, 1, &fence) != VK_SUCCESS) return false;
        VkSubmitInfo submit{};
        submit.sType = VK_STRUCTURE_TYPE_SUBMIT_INFO;
        submit.commandBufferCount = 1;
        submit.pCommandBuffers = &command;
        VkResult result = vkQueueSubmit(queue, 1, &submit, fence);
        if (result != VK_SUCCESS) queue_failure("submit", result);
        result = vkWaitForFences(device, 1, &fence, VK_TRUE, 1000000000ULL);
        if (result != VK_SUCCESS) queue_failure("fence (plazo 1 segundo)", result);
        std::array<std::uint32_t, 1024> output{};
        std::memcpy(output.data(), copy ? destination.mapped : resource.mapped, 4096);
        for (const auto word : output) if (word != 0x13579BDFU) return false;
        return true;
    }
};
bool transfer_workload(VkPhysicalDevice physical, VkDevice device, std::uint32_t family, bool reuse,
                       unsigned cycles = 64) {
    VkQueue queue = VK_NULL_HANDLE;
    vkGetDeviceQueue(device, family, 0, &queue);
    if (reuse) {
        Transfer reused{device};
        if (!reused.initialize(physical, family)) return false;
        for (unsigned cycle = 0; cycle < cycles; ++cycle) if (!reused.run(queue)) return false;
    } else {
        for (unsigned cycle = 0; cycle < cycles; ++cycle) {
            Transfer fresh{device};
            if (!fresh.initialize(physical, family) || !fresh.run(queue)) return false;
        }
    }
    return true;
}
bool transfer_cycles(VkPhysicalDevice physical, VkDevice device, std::uint32_t family) {
    if (!transfer_workload(physical, device, family, false) ||
        !transfer_workload(physical, device, family, true)) return false;
    std::puts("VERDE [V-04]: 128 envios con fence y patron correcto; 64 recreados frente a 1 reutilizado");
    return true;
}
bool copy_cycles(VkPhysicalDevice physical, VkDevice device, std::uint32_t family) {
    VkQueue queue = VK_NULL_HANDLE;
    vkGetDeviceQueue(device, family, 0, &queue);
    Transfer transfer{device};
    if (!transfer.initialize(physical, family, true)) return false;
    for (unsigned cycle = 0; cycle < 64; ++cycle) if (!transfer.run(queue)) return false;
    std::puts("VERDE [V-05]: 64 fill/copy con barrera transferencia-lectura, fence y 1024 palabras verificadas");
    return true;
}
bool memory_cycles(VkPhysicalDevice physical, VkDevice device) {
    for (unsigned cycle = 0; cycle < 64; ++cycle) {
        HostBuffer resource{device};
        if (!resource.initialize(physical, VK_BUFFER_USAGE_TRANSFER_SRC_BIT)) return false;
        std::array<std::uint8_t, 4096> pattern{};
        for (std::size_t i = 0; i < pattern.size(); ++i)
            pattern[i] = static_cast<std::uint8_t>((i + cycle) % 251);
        std::memcpy(resource.mapped, pattern.data(), pattern.size());
        if (std::memcmp(resource.mapped, pattern.data(), pattern.size()) != 0) return false;
    }
    std::puts("VERDE [V-02]: 64 ciclos buffer/memoria coherente, patron CPU y liberacion ordenada");
    return true;
}
}

int main(int argc, char** argv) {
    const bool late_error = argc == 2 && std::strcmp(argv[1], "--control-error-final") == 0;
    const bool measure = argc == 2 && std::strcmp(argv[1], "--medir") == 0;
    const bool missing_dependency = argc == 2 && std::strcmp(argv[1], "--control-sin-barrera") == 0;
    if (argc != 1 && !late_error && !measure && !missing_dependency) return 2;
    if (measure && !ultra::lab::measurement_allowed()) return 2;
    std::setvbuf(stdout, nullptr, _IONBF, 0);
    Evidence evidence;
    VkDebugUtilsMessengerCreateInfoEXT debug{};
    debug.sType = VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT;
    debug.messageSeverity = VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT;
    debug.messageType = VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT;
    debug.pfnUserCallback = message;
    debug.pUserData = &evidence;
    const VkValidationFeatureEnableEXT synchronization = VK_VALIDATION_FEATURE_ENABLE_SYNCHRONIZATION_VALIDATION_EXT;
    VkValidationFeaturesEXT validation{};
    validation.sType = VK_STRUCTURE_TYPE_VALIDATION_FEATURES_EXT;
    validation.pNext = &debug;
    validation.enabledValidationFeatureCount = 1;
    validation.pEnabledValidationFeatures = &synchronization;
    const char* layer = "VK_LAYER_KHRONOS_validation";
    const char* extensions[] = {VK_EXT_DEBUG_UTILS_EXTENSION_NAME, VK_EXT_VALIDATION_FEATURES_EXTENSION_NAME};
    VkInstanceCreateInfo instance_info{};
    instance_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    if (!measure) {
        instance_info.pNext = &validation;
        instance_info.enabledLayerCount = 1;
        instance_info.ppEnabledLayerNames = &layer;
        instance_info.enabledExtensionCount = 2;
        instance_info.ppEnabledExtensionNames = extensions;
    }
    VkInstance instance = VK_NULL_HANDLE;
    const VkResult instance_result = vkCreateInstance(&instance_info, nullptr, &instance);
    if (instance_result != VK_SUCCESS) {
        std::fprintf(stderr, "ROJO vkCreateInstance=%d\n", static_cast<int>(instance_result));
        return 1;
    }
    std::puts(measure ? "VULKAN MEDICION: sin capas; 2048 envios de 4096 bytes por ruta; fence y comprobacion incluidas"
                      : "VULKAN validacion de sincronizacion solicitada explicitamente");
    auto create_debug = reinterpret_cast<PFN_vkCreateDebugUtilsMessengerEXT>(vkGetInstanceProcAddr(instance, "vkCreateDebugUtilsMessengerEXT"));
    auto destroy_debug = reinterpret_cast<PFN_vkDestroyDebugUtilsMessengerEXT>(vkGetInstanceProcAddr(instance, "vkDestroyDebugUtilsMessengerEXT"));
    VkDebugUtilsMessengerEXT messenger = VK_NULL_HANDLE;
    if (!measure && (!create_debug || !destroy_debug || create_debug(instance, &debug, nullptr, &messenger) != VK_SUCCESS)) {
        vkDestroyInstance(instance, nullptr); return 1;
    }
    bool passed = false;
    VkDevice device = VK_NULL_HANDLE;
    std::uint32_t count = 0;
    if (vkEnumeratePhysicalDevices(instance, &count, nullptr) == VK_SUCCESS && count > 0) {
        std::vector<VkPhysicalDevice> physical(count);
        if (vkEnumeratePhysicalDevices(instance, &count, physical.data()) == VK_SUCCESS) {
            VkPhysicalDeviceProperties properties{};
            vkGetPhysicalDeviceProperties(physical[0], &properties);
            std::printf("VULKAN dispositivo=%s tipo=%u\n", properties.deviceName, static_cast<unsigned>(properties.deviceType));
            // Reject a real GPU even if an operator forgot to isolate the process.
            if (properties.deviceType == VK_PHYSICAL_DEVICE_TYPE_CPU) {
                std::uint32_t queues = 0;
                vkGetPhysicalDeviceQueueFamilyProperties(physical[0], &queues, nullptr);
                std::vector<VkQueueFamilyProperties> families(queues);
                vkGetPhysicalDeviceQueueFamilyProperties(physical[0], &queues, families.data());
                for (std::uint32_t index = 0; index < queues; ++index) {
                    if (families[index].queueCount == 0 ||
                        (families[index].queueFlags & (VK_QUEUE_TRANSFER_BIT | VK_QUEUE_GRAPHICS_BIT |
                                                      VK_QUEUE_COMPUTE_BIT)) == 0) continue;
                    const float priority = 1.0F;
                    VkDeviceQueueCreateInfo queue{};
                    queue.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
                    queue.queueFamilyIndex = index; queue.queueCount = 1; queue.pQueuePriorities = &priority;
                    VkDeviceCreateInfo info{};
                    info.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
                    info.queueCreateInfoCount = 1; info.pQueueCreateInfos = &queue;
                    if (vkCreateDevice(physical[0], &info, nullptr, &device) != VK_SUCCESS) break;
                    if (missing_dependency) {
                        // Record and discard only: never submit this deliberately invalid dependency.
                        Transfer rejected{device};
                        evidence.expecting_dependency.store(true);
                        const bool recorded = rejected.initialize(physical[0], index, true, false);
                        evidence.expecting_dependency.store(false);
                        passed = recorded && evidence.errors.load() == 1 && evidence.expected.load() == 1;
                        break;
                    }
                    if (measure) {
                        passed = ultra::lab::compare("Vulkan", [&](bool reuse) {
                            return transfer_workload(physical[0], device, index, reuse, 2048);
                        });
                        break;
                    }
                    VkBufferCreateInfo buffer_info{};
                    buffer_info.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
                    buffer_info.size = 4096;
                    buffer_info.usage = VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
                    buffer_info.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
                    VkBuffer buffer = VK_NULL_HANDLE;
                    const bool valid = vkCreateBuffer(device, &buffer_info, nullptr, &buffer) == VK_SUCCESS;
                    if (valid) vkDestroyBuffer(device, buffer, nullptr);
                    const bool memory_valid = valid && memory_cycles(physical[0], device) &&
                        transfer_cycles(physical[0], device, index) &&
                        copy_cycles(physical[0], device, index);
                    const unsigned before = evidence.errors.load();
                    buffer_info.size = 0;
                    buffer = VK_NULL_HANDLE;
                    evidence.expecting_size.store(true);
                    const VkResult invalid = vkCreateBuffer(device, &buffer_info, nullptr, &buffer);
                    evidence.expecting_size.store(false);
                    // Only this invalid call is aborted before driver dispatch.
                    if (invalid == VK_SUCCESS) vkDestroyBuffer(device, buffer, nullptr);
                    passed = memory_valid && before == 0 && evidence.errors.load() == 1 && evidence.expected.load() == 1
                        && invalid == VK_ERROR_VALIDATION_FAILED_EXT;
                    break;
                }
            }
        }
    }
    if (late_error && device != VK_NULL_HANDLE) {
        // Regression control: an error after the preliminary verdict must change the exit status.
        VkBufferCreateInfo invalid_info{};
        invalid_info.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
        invalid_info.usage = VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
        invalid_info.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
        VkBuffer rejected = VK_NULL_HANDLE;
        const VkResult result = vkCreateBuffer(device, &invalid_info, nullptr, &rejected);
        if (result == VK_SUCCESS) vkDestroyBuffer(device, rejected, nullptr);
        if (result != VK_ERROR_VALIDATION_FAILED_EXT) passed = false;
    }
    if (device != VK_NULL_HANDLE) vkDestroyDevice(device, nullptr);
    if (messenger != VK_NULL_HANDLE) destroy_debug(instance, messenger, nullptr);
    vkDestroyInstance(instance, nullptr);
    // The pNext callback remains active for instance teardown; do not pass before it.
    const unsigned required = measure ? 0U : 1U;
    passed = passed && evidence.errors.load() == required && evidence.expected.load() == required;
    std::printf("VULKAN errores=%u esperados=%u inesperados=%u\n", evidence.errors.load(),
        evidence.expected.load(), evidence.errors.load() - evidence.expected.load());
    std::puts(passed ? (measure ? "VERDE: medicion Vulkan completada con datos correctos" :
        (missing_dependency ? "VERDE [V-06]: barrera ausente detectada al grabar; cero envios" :
         "VERDE: buffer valido y llamada invalida bloqueada por validacion")) : "ROJO: resultado Vulkan inesperado");
    return passed ? 0 : 1;
}
