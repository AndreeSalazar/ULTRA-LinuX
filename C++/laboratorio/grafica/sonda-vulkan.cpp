// SPDX-License-Identifier: GPL-2.0-only
// CPU-only laboratory: validation callback aborts the invalid call before dispatch.
#include <vulkan/vulkan.h>
#include <atomic>
#include <array>
#include <cstdio>
#include <cstdint>
#include <cstring>
#include <vector>

namespace {
struct Evidence {
    std::atomic<unsigned> errors{0}, expected{0};
    std::atomic<bool> expecting_size{false};
};
VKAPI_ATTR VkBool32 VKAPI_CALL message(VkDebugUtilsMessageSeverityFlagBitsEXT severity,
    VkDebugUtilsMessageTypeFlagsEXT, const VkDebugUtilsMessengerCallbackDataEXT* data, void* user) {
    auto& evidence = *static_cast<Evidence*>(user);
    if ((severity & VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT) != 0) {
        ++evidence.errors;
        const char* id = data->pMessageIdName ? data->pMessageIdName : "sin-VUID";
        if (evidence.expecting_size.load() &&
            std::strcmp(id, "VUID-VkBufferCreateInfo-size-00912") == 0) ++evidence.expected;
        std::printf("VALIDACION %s\n", id);
        return VK_TRUE;
    }
    return VK_FALSE;
}
// No queue work: the bound allocation is used only through host-coherent mapping.
// Cleanup also runs on allocation/binding/mapping failure, in dependency order.
struct HostBuffer {
    VkDevice device;
    VkBuffer buffer = VK_NULL_HANDLE;
    VkDeviceMemory memory = VK_NULL_HANDLE;
    void* mapped = nullptr;
    explicit HostBuffer(VkDevice owner) : device(owner) {}
    HostBuffer(const HostBuffer&) = delete;
    HostBuffer& operator=(const HostBuffer&) = delete;
    ~HostBuffer() {
        if (mapped) vkUnmapMemory(device, memory);
        if (buffer) vkDestroyBuffer(device, buffer, nullptr);
        if (memory) vkFreeMemory(device, memory, nullptr);
    }
};
bool memory_cycles(VkPhysicalDevice physical, VkDevice device) {
    VkPhysicalDeviceMemoryProperties types{};
    vkGetPhysicalDeviceMemoryProperties(physical, &types);
    for (unsigned cycle = 0; cycle < 64; ++cycle) {
        HostBuffer resource{device};
        VkBufferCreateInfo info{};
        info.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
        info.size = 4096;
        info.usage = VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
        info.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
        if (vkCreateBuffer(device, &info, nullptr, &resource.buffer) != VK_SUCCESS) return false;
        VkMemoryRequirements requirements{};
        vkGetBufferMemoryRequirements(device, resource.buffer, &requirements);
        // Keep the test bounded even if a different software driver changes its requirements.
        if (requirements.size > 1024 * 1024) return false;
        std::uint32_t type = types.memoryTypeCount;
        constexpr VkMemoryPropertyFlags flags = VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT |
                                               VK_MEMORY_PROPERTY_HOST_COHERENT_BIT;
        for (std::uint32_t i = 0; i < types.memoryTypeCount; ++i) {
            if ((requirements.memoryTypeBits & (std::uint32_t{1} << i)) != 0 &&
                (types.memoryTypes[i].propertyFlags & flags) == flags) { type = i; break; }
        }
        if (type == types.memoryTypeCount) return false;
        VkMemoryAllocateInfo allocation{};
        allocation.sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO;
        allocation.allocationSize = requirements.size;
        allocation.memoryTypeIndex = type;
        if (vkAllocateMemory(device, &allocation, nullptr, &resource.memory) != VK_SUCCESS ||
            vkBindBufferMemory(device, resource.buffer, resource.memory, 0) != VK_SUCCESS ||
            vkMapMemory(device, resource.memory, 0, 4096, 0, &resource.mapped) != VK_SUCCESS) return false;
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
    if (argc != 1 && !late_error) return 2;
    std::setvbuf(stdout, nullptr, _IONBF, 0);
    Evidence evidence;
    VkDebugUtilsMessengerCreateInfoEXT debug{};
    debug.sType = VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT;
    debug.messageSeverity = VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT;
    debug.messageType = VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT;
    debug.pfnUserCallback = message;
    debug.pUserData = &evidence;
    const char* layer = "VK_LAYER_KHRONOS_validation";
    const char* extension = VK_EXT_DEBUG_UTILS_EXTENSION_NAME;
    VkInstanceCreateInfo instance_info{};
    instance_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    instance_info.pNext = &debug;
    instance_info.enabledLayerCount = 1;
    instance_info.ppEnabledLayerNames = &layer;
    instance_info.enabledExtensionCount = 1;
    instance_info.ppEnabledExtensionNames = &extension;
    VkInstance instance = VK_NULL_HANDLE;
    const VkResult instance_result = vkCreateInstance(&instance_info, nullptr, &instance);
    if (instance_result != VK_SUCCESS) {
        std::fprintf(stderr, "ROJO vkCreateInstance=%d\n", static_cast<int>(instance_result));
        return 1;
    }
    auto create_debug = reinterpret_cast<PFN_vkCreateDebugUtilsMessengerEXT>(vkGetInstanceProcAddr(instance, "vkCreateDebugUtilsMessengerEXT"));
    auto destroy_debug = reinterpret_cast<PFN_vkDestroyDebugUtilsMessengerEXT>(vkGetInstanceProcAddr(instance, "vkDestroyDebugUtilsMessengerEXT"));
    VkDebugUtilsMessengerEXT messenger = VK_NULL_HANDLE;
    if (!create_debug || !destroy_debug || create_debug(instance, &debug, nullptr, &messenger) != VK_SUCCESS) {
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
                    if (families[index].queueCount == 0) continue;
                    const float priority = 1.0F;
                    VkDeviceQueueCreateInfo queue{};
                    queue.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
                    queue.queueFamilyIndex = index; queue.queueCount = 1; queue.pQueuePriorities = &priority;
                    VkDeviceCreateInfo info{};
                    info.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
                    info.queueCreateInfoCount = 1; info.pQueueCreateInfos = &queue;
                    if (vkCreateDevice(physical[0], &info, nullptr, &device) != VK_SUCCESS) break;
                    VkBufferCreateInfo buffer_info{};
                    buffer_info.sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO;
                    buffer_info.size = 4096;
                    buffer_info.usage = VK_BUFFER_USAGE_TRANSFER_SRC_BIT;
                    buffer_info.sharingMode = VK_SHARING_MODE_EXCLUSIVE;
                    VkBuffer buffer = VK_NULL_HANDLE;
                    const bool valid = vkCreateBuffer(device, &buffer_info, nullptr, &buffer) == VK_SUCCESS;
                    if (valid) vkDestroyBuffer(device, buffer, nullptr);
                    const bool memory_valid = valid && memory_cycles(physical[0], device);
                    const unsigned before = evidence.errors.load();
                    buffer_info.size = 0;
                    buffer = VK_NULL_HANDLE;
                    evidence.expecting_size.store(true);
                    const VkResult invalid = vkCreateBuffer(device, &buffer_info, nullptr, &buffer);
                    evidence.expecting_size.store(false);
                    // No memory is allocated, no queue submits or shader dispatches occur.
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
    destroy_debug(instance, messenger, nullptr);
    vkDestroyInstance(instance, nullptr);
    // The pNext callback remains active for instance teardown; do not pass before it.
    passed = passed && evidence.errors.load() == 1 && evidence.expected.load() == 1;
    std::printf("VULKAN errores=%u esperados=%u inesperados=%u\n", evidence.errors.load(),
        evidence.expected.load(), evidence.errors.load() - evidence.expected.load());
    std::puts(passed ? "VERDE: buffer valido y llamada invalida bloqueada por validacion" : "ROJO: resultado Vulkan inesperado");
    return passed ? 0 : 1;
}
